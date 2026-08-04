//! Host-facing controller runtime: app-data mappings + midir + MappingSession.

use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};

use midir::{Ignore, MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};
use serde::Serialize;
use thiserror::Error;

use crate::bundle::{load_bundle, Bundle};
use crate::error::{LoadError, RuntimeError};
use crate::midi::{match_device, MidiIdentity};
use crate::session::{ActionPublish, MappingSession, MidiOut};

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
    pub name: String,
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

/// Glue midir + app-data mapping bundles for a host (Tauri / WASM).
pub struct ControllerEngine {
    app_dir: PathBuf,
    shipped_dir: PathBuf,
    /// mapping id → bundle root under app-data
    catalog: HashMap<String, PathBuf>,
    known_input_ports: HashSet<String>,
    /// Ports already offered this appearance (cleared when port disappears).
    offered_ports: HashSet<String>,
    events: VecDeque<ControllerEvent>,
    midi_tx: Sender<Vec<u8>>,
    midi_rx: Receiver<Vec<u8>>,
    attached: Option<Attached>,
}

impl ControllerEngine {
    pub fn open(
        app_mappings_dir: impl Into<PathBuf>,
        shipped_mappings_dir: impl Into<PathBuf>,
    ) -> Self {
        let (midi_tx, midi_rx) = mpsc::channel();
        Self {
            app_dir: app_mappings_dir.into(),
            shipped_dir: shipped_mappings_dir.into(),
            catalog: HashMap::new(),
            known_input_ports: HashSet::new(),
            offered_ports: HashSet::new(),
            events: VecDeque::new(),
            midi_tx,
            midi_rx,
            attached: None,
        }
    }

    pub fn app_dir(&self) -> &Path {
        &self.app_dir
    }

    pub fn shipped_dir(&self) -> &Path {
        &self.shipped_dir
    }

    /// Copy each shipped mapping into app-data when the destination id is missing.
    pub fn ensure_seeded(&mut self) -> Result<(), EngineError> {
        fs::create_dir_all(&self.app_dir)?;
        if !self.shipped_dir.is_dir() {
            self.rescan_catalog()?;
            return Ok(());
        }
        for entry in fs::read_dir(&self.shipped_dir)? {
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
        let src = self.shipped_dir.join(mapping_id);
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
        if !self.shipped_dir.is_dir() {
            return Ok(());
        }
        let ids: Vec<String> = fs::read_dir(&self.shipped_dir)?
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
                Ok(_bundle) => {
                    let folder = entry.file_name().to_string_lossy().into_owned();
                    self.catalog.insert(folder, path);
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
        let mut out = Vec::new();
        for (id, path) in &self.catalog {
            let bundle = load_bundle(path)?;
            out.push(MappingInfo {
                id: id.clone(),
                device_id: bundle.device.id,
                name: bundle.device.name,
                midi_name_contains: bundle.device.midi_name_contains,
                attached: attached_id == Some(id.as_str()),
            });
        }
        out.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(out)
    }

    pub fn list_devices(&self) -> Result<Vec<DeviceInfo>, EngineError> {
        let mut out = Vec::new();
        let input = MidiInput::new("rust-mixer-controller-in")
            .map_err(|e| EngineError::Midi(e.to_string()))?;
        for port in input.ports() {
            let port_name = input
                .port_name(&port)
                .map_err(|e| EngineError::Midi(e.to_string()))?;
            let matched = self.match_port(&port_name);
            out.push(DeviceInfo {
                port_name,
                direction: DeviceDirection::Input,
                matched_mapping_id: matched,
            });
        }
        let output = MidiOutput::new("rust-mixer-controller-out")
            .map_err(|e| EngineError::Midi(e.to_string()))?;
        for port in output.ports() {
            let port_name = output
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

    /// Diff input ports; push [`ControllerEvent::MappingOffer`] for new matches.
    pub fn poll_devices(&mut self) -> Result<(), EngineError> {
        let input = MidiInput::new("rust-mixer-controller-poll")
            .map_err(|e| EngineError::Midi(e.to_string()))?;
        let mut current = HashSet::new();
        for port in input.ports() {
            let name = input
                .port_name(&port)
                .map_err(|e| EngineError::Midi(e.to_string()))?;
            current.insert(name);
        }

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
                self.disable_mapping(&id)?;
            }
        }

        for name in &current {
            if self.known_input_ports.contains(name) {
                continue;
            }
            if self.offered_ports.contains(name) {
                continue;
            }
            if self.attached.as_ref().is_some_and(|a| &a.port_name == name) {
                continue;
            }
            if let Some(mapping_id) = self.match_port(name) {
                let device_name = self
                    .catalog
                    .get(&mapping_id)
                    .and_then(|p| load_bundle(p).ok())
                    .map(|b| b.device.name)
                    .unwrap_or_else(|| mapping_id.clone());
                self.offered_ports.insert(name.clone());
                self.events.push_back(ControllerEvent::MappingOffer {
                    mapping_id,
                    device_name,
                    port_name: name.clone(),
                });
            }
        }

        self.known_input_ports = current;
        Ok(())
    }

    pub fn take_events(&mut self) -> Vec<ControllerEvent> {
        self.events.drain(..).collect()
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
            .cloned()
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

        let midi_in = MidiInput::new("rust-mixer-controller-in")
            .map_err(|e| EngineError::Midi(e.to_string()))?;
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
                "rust-mixer-map",
                move |_stamp, message, _| {
                    let _ = tx.send(message.to_vec());
                },
                (),
            )
            .map_err(|e| EngineError::Midi(e.to_string()))?;

        let mut output = open_matching_output(&bundle, &port_name).ok().flatten();
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
        Ok(())
    }

    pub fn pump(&mut self, bus: &mut impl ActionPublish) {
        loop {
            match self.midi_rx.try_recv() {
                Ok(bytes) => {
                    if let Some(attached) = self.attached.as_mut() {
                        attached.session.handle_midi(&bytes, bus);
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
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

    fn match_port(&self, port_name: &str) -> Option<String> {
        let identity = MidiIdentity {
            usb_vid: None,
            usb_pid: None,
            port_name: port_name.to_string(),
        };
        for (id, path) in &self.catalog {
            if let Ok(bundle) = load_bundle(path) {
                if match_device(
                    &identity,
                    bundle.device.usb_vid,
                    bundle.device.usb_pid,
                    &bundle.device.midi_name_contains,
                ) {
                    return Some(id.clone());
                }
            }
        }
        None
    }

    fn find_matching_input_port(&self, bundle: &Bundle) -> Result<Option<String>, EngineError> {
        let input = MidiInput::new("rust-mixer-controller-find")
            .map_err(|e| EngineError::Midi(e.to_string()))?;
        for port in input.ports() {
            let name = input
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
    fn publish_library_evt(
        &mut self,
        _origin: library_api::Origin,
        _kind: library_api::Kind,
        _body: library_api::EvtBody,
    ) {
    }
}

fn open_matching_output(
    bundle: &Bundle,
    input_port_name: &str,
) -> Result<Option<MidiOutputConnection>, EngineError> {
    let output = MidiOutput::new("rust-mixer-controller-out")
        .map_err(|e| EngineError::Midi(e.to_string()))?;
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
        .connect(&port, "rust-mixer-map-out")
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
