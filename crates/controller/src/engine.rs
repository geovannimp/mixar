//! Host-facing controller runtime: app-data mappings + midir + MappingSession.

use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};

use midir::{Ignore, MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};
use serde::Serialize;
use thiserror::Error;

use crate::bundle::{load_bundle, MappingBundle};
use crate::error::{LoadError, RuntimeError};
use crate::midi::{match_device, MidiIdentity};
use crate::session::{ActionPublish, MappingSession, MidiOut};
use engine_api::PadMode;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Load(#[from] LoadError),
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error("mapping `{0}` not found in app-data catalog")]
    UnknownMapping(String),
    #[error("shipped mapping `{0}` not found")]
    UnknownShipped(String),
    #[error("no live MIDI port matches mapping `{0}`")]
    NoMatchingPort(String),
    #[error("MIDI I/O: {0}")]
    Midi(String),
}

#[derive(Clone, Debug, Serialize)]
pub struct MappingInfo {
    /// App-data folder name (stable for update paths).
    pub id: String,
    /// `device.toml` id (e.g. `pioneer.ddj-400`).
    pub device_id: String,
    pub vendor_name: String,
    pub product_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub midi_name_contains: Vec<String>,
    pub attached: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct DeviceInfo {
    pub port_name: String,
    pub direction: DeviceDirection,
    pub matched_mapping_id: Option<String>,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeviceDirection {
    Input,
    Output,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ControllerEvent {
    MappingOffer {
        mapping_id: String,
        /// `device.toml` id (trust key), e.g. `pioneer.ddj-400`.
        device_id: String,
        device_name: String,
        port_name: String,
    },
    MappingAttached {
        mapping_id: String,
        port_name: String,
    },
    MappingDetached {
        mapping_id: String,
    },
}

struct Attached {
    mapping_id: String,
    port_name: String,
    session: MappingSession,
    _input: MidiInputConnection<()>,
    output: Option<MidiOutputConnection>,
}

struct MidiSink<'a> {
    out: &'a mut Option<MidiOutputConnection>,
}

impl MidiOut for MidiSink<'_> {
    fn send(&mut self, bytes: &[u8]) {
        if let Some(out) = self.out.as_mut() {
            let _ = out.send(bytes);
        }
    }
}

struct CatalogEntry {
    path: PathBuf,
    device_id: String,
    vendor_name: String,
    product_name: String,
    description: Option<String>,
    usb_vid: Option<u16>,
    usb_pid: Option<u16>,
    midi_name_contains: Vec<String>,
}

impl CatalogEntry {
    fn display_name(&self) -> String {
        [self.vendor_name.as_str(), self.product_name.as_str()]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Glue midir + app-data mapping bundles for a host (Tauri / WASM).
///
/// Port listing uses long-lived [`Self::enum_in`] / [`Self::enum_out`].
/// Each attach still opens its own midir clients — `connect` consumes them —
/// so multiple controllers can each own a connection pair later.
pub struct ControllerEngine {
    app_name: String,
    app_dir: PathBuf,
    shipped_mappings_dir: PathBuf,
    /// mapping folder id → cached device identity + path
    catalog: HashMap<String, CatalogEntry>,
    known_input_ports: HashSet<String>,
    /// Ports already event-emitted this appearance (cleared when port disappears).
    offered_ports: HashSet<String>,
    /// `device.toml` ids the host trusts — auto-attach on match, no offer.
    trusted_device_ids: HashSet<String>,
    /// Snapshot of unmatched mapped ports — no MIDI I/O to read.
    pending_offers_cache: Vec<ControllerEvent>,
    events: VecDeque<ControllerEvent>,
    midi_tx: Sender<Vec<u8>>,
    midi_rx: Receiver<Vec<u8>>,
    /// Enumeration-only midir clients (never `connect`ed). Lazy so seed/open works without ALSA.
    enum_in: Option<MidiInput>,
    enum_out: Option<MidiOutput>,
    attached: Option<Attached>,
}

impl ControllerEngine {
    /// Open and seed app-data mappings from the shipped catalog when missing.
    ///
    /// `app_name` labels midir clients in the system MIDI list (e.g. `"Mixar"`).
    pub fn open(
        app_name: impl Into<String>,
        app_mappings_dir: impl Into<PathBuf>,
        shipped_mappings_dir: impl Into<PathBuf>,
    ) -> Result<Self, EngineError> {
        let (midi_tx, midi_rx) = mpsc::channel();
        let mut this = Self {
            app_name: app_name.into(),
            app_dir: app_mappings_dir.into(),
            shipped_mappings_dir: shipped_mappings_dir.into(),
            catalog: HashMap::new(),
            known_input_ports: HashSet::new(),
            offered_ports: HashSet::new(),
            trusted_device_ids: HashSet::new(),
            pending_offers_cache: Vec::new(),
            events: VecDeque::new(),
            midi_tx,
            midi_rx,
            enum_in: None,
            enum_out: None,
            attached: None,
        };
        this.ensure_seeded()?;
        Ok(this)
    }

    /// Replace the in-memory trust allow-list (`device.toml` ids). Host persists separately.
    pub fn set_trusted_device_ids(&mut self, ids: impl IntoIterator<Item = String>) {
        self.trusted_device_ids = ids.into_iter().collect();
    }

    fn ensure_enum_clients(&mut self) -> Result<(), EngineError> {
        if self.enum_in.is_none() {
            self.enum_in =
                Some(MidiInput::new(&self.app_name).map_err(|e| EngineError::Midi(e.to_string()))?);
        }
        if self.enum_out.is_none() {
            let name = format!("{} out", self.app_name);
            self.enum_out =
                Some(MidiOutput::new(&name).map_err(|e| EngineError::Midi(e.to_string()))?);
        }
        Ok(())
    }

    pub fn app_name(&self) -> &str {
        &self.app_name
    }

    pub fn app_dir(&self) -> &Path {
        &self.app_dir
    }

    pub fn shipped_mappings_dir(&self) -> &Path {
        &self.shipped_mappings_dir
    }

    /// Copy each shipped mapping into app-data when the destination id is missing.
    pub fn ensure_seeded(&mut self) -> Result<(), EngineError> {
        fs::create_dir_all(&self.app_dir)?;
        if !self.shipped_mappings_dir.is_dir() {
            self.rescan_catalog()?;
            return Ok(());
        }
        for entry in fs::read_dir(&self.shipped_mappings_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let name = entry.file_name();
            let dest = self.app_dir.join(&name);
            if dest.exists() {
                continue;
            }
            copy_dir_all(&entry.path(), &dest)?;
        }
        self.rescan_catalog()
    }

    pub fn update_mapping(&mut self, mapping_id: &str) -> Result<(), EngineError> {
        let src = self.shipped_mappings_dir.join(mapping_id);
        if !src.is_dir() {
            return Err(EngineError::UnknownShipped(mapping_id.to_string()));
        }
        let dest = self.app_dir.join(mapping_id);
        if dest.exists() {
            fs::remove_dir_all(&dest)?;
        }
        copy_dir_all(&src, &dest)?;
        self.rescan_catalog()?;
        if self
            .attached
            .as_ref()
            .is_some_and(|a| a.mapping_id == mapping_id)
        {
            let port = self
                .attached
                .as_ref()
                .map(|a| a.port_name.clone())
                .expect("attached");
            self.disable_mapping(mapping_id)?;
            let _ = self.enable_mapping(mapping_id, Some(&port));
        }
        Ok(())
    }

    pub fn update_all_mappings(&mut self) -> Result<(), EngineError> {
        if !self.shipped_mappings_dir.is_dir() {
            return Ok(());
        }
        let ids: Vec<String> = fs::read_dir(&self.shipped_mappings_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        for id in ids {
            self.update_mapping(&id)?;
        }
        Ok(())
    }

    pub fn rescan_catalog(&mut self) -> Result<(), EngineError> {
        self.catalog.clear();
        fs::create_dir_all(&self.app_dir)?;
        if !self.app_dir.is_dir() {
            return Ok(());
        }
        for entry in fs::read_dir(&self.app_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let path = entry.path();
            match load_bundle(&path) {
                Ok(bundle) => {
                    let folder = entry.file_name().to_string_lossy().into_owned();
                    self.catalog.insert(
                        folder,
                        CatalogEntry {
                            path,
                            device_id: bundle.device.id,
                            vendor_name: bundle.device.vendor_name,
                            product_name: bundle.device.product_name,
                            description: bundle.device.description,
                            usb_vid: bundle.device.usb_vid,
                            usb_pid: bundle.device.usb_pid,
                            midi_name_contains: bundle.device.midi_name_contains,
                        },
                    );
                }
                Err(err) => {
                    log::warn!("skip invalid mapping {}: {err}", path.display());
                }
            }
        }
        Ok(())
    }

    pub fn list_mappings(&self) -> Result<Vec<MappingInfo>, EngineError> {
        let attached_id = self.attached.as_ref().map(|a| a.mapping_id.as_str());
        let mut out: Vec<MappingInfo> = self
            .catalog
            .iter()
            .map(|(id, entry)| MappingInfo {
                id: id.clone(),
                device_id: entry.device_id.clone(),
                vendor_name: entry.vendor_name.clone(),
                product_name: entry.product_name.clone(),
                description: entry.description.clone(),
                midi_name_contains: entry.midi_name_contains.clone(),
                attached: attached_id == Some(id.as_str()),
            })
            .collect();
        out.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(out)
    }

    pub fn list_devices(&mut self) -> Result<Vec<DeviceInfo>, EngineError> {
        self.ensure_enum_clients()?;
        let enum_in = self.enum_in.as_ref().expect("enum_in after ensure");
        let enum_out = self.enum_out.as_ref().expect("enum_out after ensure");
        let mut out = Vec::new();
        for port in enum_in.ports() {
            let port_name = enum_in
                .port_name(&port)
                .map_err(|e| EngineError::Midi(e.to_string()))?;
            let matched = self.match_port(&port_name);
            out.push(DeviceInfo {
                port_name,
                direction: DeviceDirection::Input,
                matched_mapping_id: matched,
            });
        }
        for port in enum_out.ports() {
            let port_name = enum_out
                .port_name(&port)
                .map_err(|e| EngineError::Midi(e.to_string()))?;
            let matched = self.match_port(&port_name);
            out.push(DeviceInfo {
                port_name,
                direction: DeviceDirection::Output,
                matched_mapping_id: matched,
            });
        }
        Ok(out)
    }

    /// Input port names via the long-lived enumeration client (created once).
    pub fn list_input_port_names(&mut self) -> Result<HashSet<String>, EngineError> {
        self.ensure_enum_clients()?;
        let enum_in = self.enum_in.as_ref().expect("enum_in after ensure");
        let mut current = HashSet::new();
        for port in enum_in.ports() {
            let name = enum_in
                .port_name(&port)
                .map_err(|e| EngineError::Midi(e.to_string()))?;
            current.insert(name);
        }
        Ok(current)
    }

    /// Diff input ports; auto-attach trusted matches, else offer for new ports.
    pub fn apply_input_ports(&mut self, current: HashSet<String>) {
        for gone in self
            .known_input_ports
            .difference(&current)
            .cloned()
            .collect::<Vec<_>>()
        {
            self.offered_ports.remove(&gone);
            if self.attached.as_ref().is_some_and(|a| a.port_name == gone) {
                let id = self
                    .attached
                    .as_ref()
                    .map(|a| a.mapping_id.clone())
                    .expect("attached");
                let _ = self.disable_mapping(&id);
            }
        }

        for name in &current {
            if self.attached.as_ref().is_some_and(|a| &a.port_name == name) {
                continue;
            }
            let Some(mapping_id) = self.match_port(name) else {
                continue;
            };
            let (device_id, device_name) = self
                .catalog
                .get(&mapping_id)
                .map(|e| (e.device_id.clone(), e.display_name()))
                .unwrap_or_else(|| (mapping_id.clone(), mapping_id.clone()));

            if self.trusted_device_ids.contains(&device_id) {
                // Retry each poll until MIDI open succeeds; do not emit offers.
                let _ = self.enable_mapping(&mapping_id, Some(name));
                continue;
            }

            if self.known_input_ports.contains(name) {
                continue;
            }
            if self.offered_ports.contains(name) {
                continue;
            }
            self.offered_ports.insert(name.clone());
            self.events.push_back(ControllerEvent::MappingOffer {
                mapping_id,
                device_id,
                device_name,
                port_name: name.clone(),
            });
        }

        self.known_input_ports = current;
        self.refresh_pending_offers_cache();
    }

    /// Enumerate + apply using the shared enumeration client.
    pub fn poll_devices(&mut self) -> Result<(), EngineError> {
        let current = self.list_input_port_names()?;
        self.apply_input_ports(current);
        Ok(())
    }

    pub fn take_events(&mut self) -> Vec<ControllerEvent> {
        self.events.drain(..).collect()
    }

    /// Cached unmatched mapped ports — no MIDI I/O (safe for FE hydrate).
    pub fn pending_offers(&self) -> Vec<ControllerEvent> {
        self.pending_offers_cache.clone()
    }

    fn refresh_pending_offers_cache(&mut self) {
        self.pending_offers_cache.clear();
        for name in &self.known_input_ports {
            if self.attached.as_ref().is_some_and(|a| &a.port_name == name) {
                continue;
            }
            let Some(mapping_id) = self.match_port(name) else {
                continue;
            };
            let (device_id, device_name) = self
                .catalog
                .get(&mapping_id)
                .map(|e| (e.device_id.clone(), e.display_name()))
                .unwrap_or_else(|| (mapping_id.clone(), mapping_id.clone()));
            if self.trusted_device_ids.contains(&device_id) {
                continue;
            }
            self.pending_offers_cache
                .push(ControllerEvent::MappingOffer {
                    mapping_id,
                    device_id,
                    device_name,
                    port_name: name.clone(),
                });
        }
    }

    /// Attach mapping to `port_name` (or first matching live input).
    pub fn enable_mapping(
        &mut self,
        mapping_id: &str,
        port_name: Option<&str>,
    ) -> Result<(), EngineError> {
        let path = self
            .catalog
            .get(mapping_id)
            .map(|e| e.path.clone())
            .ok_or_else(|| EngineError::UnknownMapping(mapping_id.to_string()))?;
        let bundle = load_bundle(&path)?;

        let port_name = if let Some(p) = port_name {
            p.to_string()
        } else {
            self.find_matching_input_port(&bundle)?
                .ok_or_else(|| EngineError::NoMatchingPort(mapping_id.to_string()))?
        };

        if let Some(attached) = &self.attached {
            if attached.mapping_id == mapping_id && attached.port_name == port_name {
                return Ok(());
            }
            let old = attached.mapping_id.clone();
            self.disable_mapping(&old)?;
        }

        // Fresh client: midir `connect` consumes MidiInput (one per attached controller).
        let map_name = format!("{} map", self.app_name);
        let midi_in = MidiInput::new(&map_name).map_err(|e| EngineError::Midi(e.to_string()))?;
        let mut midi_in = midi_in;
        midi_in.ignore(Ignore::None);
        let port = midi_in
            .ports()
            .into_iter()
            .find(|p| midi_in.port_name(p).ok().as_deref() == Some(port_name.as_str()))
            .ok_or_else(|| EngineError::NoMatchingPort(mapping_id.to_string()))?;

        let tx = self.midi_tx.clone();
        let input = midi_in
            .connect(
                &port,
                "in",
                move |_stamp, message, _| {
                    let _ = tx.send(message.to_vec());
                },
                (),
            )
            .map_err(|e| EngineError::Midi(e.to_string()))?;

        let mut output = open_matching_output(&self.app_name, &bundle, &port_name)
            .ok()
            .flatten();
        let mut session = MappingSession::from_bundle(bundle)?;
        {
            let mut sink = MidiSink { out: &mut output };
            session.on_init(&mut NullPublish, &mut sink)?;
        }
        self.attached = Some(Attached {
            mapping_id: mapping_id.to_string(),
            port_name: port_name.clone(),
            session,
            _input: input,
            output,
        });
        self.refresh_pending_offers_cache();

        self.events.push_back(ControllerEvent::MappingAttached {
            mapping_id: mapping_id.to_string(),
            port_name,
        });
        Ok(())
    }

    pub fn disable_mapping(&mut self, mapping_id: &str) -> Result<(), EngineError> {
        let Some(mut attached) = self.attached.take() else {
            return Ok(());
        };
        if attached.mapping_id != mapping_id {
            self.attached = Some(attached);
            return Ok(());
        }
        let mut sink = MidiSink {
            out: &mut attached.output,
        };
        let _ = attached.session.on_shutdown(&mut NullPublish, &mut sink);
        self.events.push_back(ControllerEvent::MappingDetached {
            mapping_id: mapping_id.to_string(),
        });
        self.refresh_pending_offers_cache();
        Ok(())
    }

    pub fn pump(&mut self, bus: &mut impl ActionPublish) {
        loop {
            match self.midi_rx.try_recv() {
                Ok(bytes) => {
                    if let Some(attached) = self.attached.as_mut() {
                        let mut sink = MidiSink {
                            out: &mut attached.output,
                        };
                        attached.session.handle_midi(&bytes, bus, &mut sink);
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }
        if let Some(attached) = self.attached.as_mut() {
            let mut sink = MidiSink {
                out: &mut attached.output,
            };
            attached.session.flush_coalesced(bus, &mut sink);
            let _ = attached.session.idle_heartbeat(bus, &mut sink);
        }
    }

    pub fn on_deck_playing(&mut self, deck: u16, playing: bool) {
        if let Some(attached) = self.attached.as_mut() {
            let mut sink = MidiSink {
                out: &mut attached.output,
            };
            attached.session.on_deck_playing(deck, playing, &mut sink);
        }
    }

    /// Mirror library hot cues into the attached mapping (pad Trigger vs Save + LEDs).
    pub fn set_deck_hot_cues(&mut self, deck: u16, cues: [Option<i32>; crate::HOT_CUE_SLOT_COUNT]) {
        if let Some(attached) = self.attached.as_mut() {
            let mut sink = MidiSink {
                out: &mut attached.output,
            };
            attached.session.set_deck_hot_cues(deck, cues, &mut sink);
        }
    }

    /// Mirror engine pad mode so MIDI `pad n` matches the UI.
    pub fn set_deck_pad_mode(&mut self, deck: u16, mode: PadMode) {
        if let Some(attached) = self.attached.as_mut() {
            let mut sink = MidiSink {
                out: &mut attached.output,
            };
            attached.session.set_deck_pad_mode(deck, mode, &mut sink);
        }
    }

    /// Push deck peak level to `vu_meter` MIDI out (no-op if mapping has none).
    pub fn set_deck_vu(&mut self, deck: u16, level: f32) {
        if let Some(attached) = self.attached.as_mut() {
            let mut sink = MidiSink {
                out: &mut attached.output,
            };
            attached.session.set_deck_vu(deck, level, &mut sink);
        }
    }

    fn match_port(&self, port_name: &str) -> Option<String> {
        let identity = MidiIdentity {
            usb_vid: None,
            usb_pid: None,
            port_name: port_name.to_string(),
        };
        for (id, entry) in &self.catalog {
            if match_device(
                &identity,
                entry.usb_vid,
                entry.usb_pid,
                &entry.midi_name_contains,
            ) {
                return Some(id.clone());
            }
        }
        None
    }

    fn find_matching_input_port(
        &mut self,
        bundle: &MappingBundle,
    ) -> Result<Option<String>, EngineError> {
        self.ensure_enum_clients()?;
        let enum_in = self.enum_in.as_ref().expect("enum_in after ensure");
        for port in enum_in.ports() {
            let name = enum_in
                .port_name(&port)
                .map_err(|e| EngineError::Midi(e.to_string()))?;
            let identity = MidiIdentity {
                usb_vid: None,
                usb_pid: None,
                port_name: name.clone(),
            };
            if match_device(
                &identity,
                bundle.device.usb_vid,
                bundle.device.usb_pid,
                &bundle.device.midi_name_contains,
            ) {
                return Ok(Some(name));
            }
        }
        Ok(None)
    }
}

struct NullPublish;

impl ActionPublish for NullPublish {
    fn publish_engine(
        &mut self,
        _origin: engine_api::Origin,
        _kind: engine_api::Kind,
        _body: engine_api::CmdBody,
    ) {
    }
    fn publish_library(
        &mut self,
        _origin: library_api::Origin,
        _kind: library_api::Kind,
        _body: library_api::EvtBody,
    ) {
    }
}

fn open_matching_output(
    app_name: &str,
    bundle: &MappingBundle,
    input_port_name: &str,
) -> Result<Option<MidiOutputConnection>, EngineError> {
    // Fresh client: midir `connect` consumes MidiOutput (one per attached controller).
    let name = format!("{app_name} map out");
    let output = MidiOutput::new(&name).map_err(|e| EngineError::Midi(e.to_string()))?;
    let ports = output.ports();
    let mut chosen = None;
    for p in &ports {
        let Ok(name) = output.port_name(p) else {
            continue;
        };
        if name == input_port_name {
            chosen = Some(p.clone());
            break;
        }
    }
    if chosen.is_none() {
        for p in &ports {
            let Ok(name) = output.port_name(p) else {
                continue;
            };
            let identity = MidiIdentity {
                usb_vid: None,
                usb_pid: None,
                port_name: name,
            };
            if match_device(
                &identity,
                bundle.device.usb_vid,
                bundle.device.usb_pid,
                &bundle.device.midi_name_contains,
            ) {
                chosen = Some(p.clone());
                break;
            }
        }
    }
    let Some(port) = chosen else {
        return Ok(None);
    };
    let conn = output
        .connect(&port, "out")
        .map_err(|e| EngineError::Midi(e.to_string()))?;
    Ok(Some(conn))
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), EngineError> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &to)?;
        } else if ty.is_file() {
            fs::copy(entry.path(), to)?;
        }
    }
    Ok(())
}
