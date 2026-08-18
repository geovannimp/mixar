//! FRB `ControllerTransport`: MIDI mapping host over shared engine/library buses.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use controller::{
    ActionPublish, ControllerEngine, ControllerEvent, DeviceDirection, DeviceInfo, MappingInfo,
    HOT_CUE_SLOT_COUNT,
};
use engine_api::{
    decode_evt_body, encode_cmd_body, CmdBody, DeckHotCue, EvtBody, Kind, Origin, PadMode,
};
use engine_core::EngineBuses;
use library::LibraryBuses;
use library_api::{
    decode_evt_body as decode_library_evt, EvtBody as LibraryEvtBody, HotCue as LibraryHotCue,
    Kind as LibraryKind, Origin as LibraryOrigin,
};

use crate::api::engine::EngineBusHandle;
use crate::api::library::LibraryBusHandle;
use crate::frb_generated::StreamSink;

/// MIDI pump cadence — never blocks on ALSA port enumeration.
const PUMP_INTERVAL: Duration = Duration::from_millis(5);
/// Hotplug / offer scan — MidiInput::new is expensive on Linux; own thread.
const DEVICE_POLL_INTERVAL: Duration = Duration::from_secs(2);

/// Mapping row for Flutter settings (mirrors Tauri `ControllerMappingInfo`).
#[derive(Clone, Debug)]
pub struct ControllerMappingInfo {
    pub id: String,
    pub device_id: String,
    pub vendor_name: String,
    pub product_name: String,
    pub description: Option<String>,
    pub midi_name_contains: Vec<String>,
    pub attached: bool,
}

/// Live MIDI port row for Flutter settings.
#[derive(Clone, Debug)]
pub struct ControllerDeviceInfo {
    pub port_name: String,
    pub direction: ControllerDeviceDirection,
    pub matched_mapping_id: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControllerDeviceDirection {
    Input,
    Output,
}

/// Discriminator for controller host events (unit enum — no freezed on Dart).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControllerEvtKind {
    MappingOffer,
    MappingAttached,
    MappingDetached,
}

/// Thin typed controller egress for Dart (no MessagePack on the Flutter side).
#[derive(Clone, Debug)]
pub struct ControllerEvt {
    pub kind: ControllerEvtKind,
    pub mapping_id: Option<String>,
    pub device_name: Option<String>,
    pub port_name: Option<String>,
}

struct HostPublish {
    engine: EngineBuses,
    library: LibraryBuses,
}

impl ActionPublish for HostPublish {
    fn publish_engine(&mut self, origin: Origin, kind: Kind, body: CmdBody) {
        let Ok(bytes) = encode_cmd_body(&body) else {
            return;
        };
        let _ = self.engine.publish_cmd(origin, kind, bytes);
    }

    fn publish_library(&mut self, origin: LibraryOrigin, kind: LibraryKind, body: LibraryEvtBody) {
        let _ = self.library.publish_evt(origin, kind, body);
    }
}

/// Host-owned MIDI mapping engine + poll/pump threads.
#[flutter_rust_bridge::frb(opaque)]
pub struct ControllerTransport {
    engine: Arc<Mutex<ControllerEngine>>,
    stop: Arc<AtomicBool>,
    threads: Mutex<Vec<JoinHandle<()>>>,
    sink: Arc<Mutex<Option<StreamSink<ControllerEvt>>>>,
}

impl Drop for ControllerTransport {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Ok(mut slot) = self.sink.lock() {
            *slot = None;
        }
        let mut threads = self.threads.lock().unwrap_or_else(|e| e.into_inner());
        for handle in threads.drain(..) {
            let _ = handle.join();
        }
    }
}

impl ControllerTransport {
    /// Open `ControllerEngine`, seed app-data mappings, start poll/pump threads.
    pub fn start(
        engine_buses: &EngineBusHandle,
        library_buses: &LibraryBusHandle,
        mappings_dir: String,
        shipped_mappings_dir: Option<String>,
    ) -> Result<Self, String> {
        let shipped = shipped_mappings_dir
            .map(PathBuf::from)
            .unwrap_or_else(resolve_shipped_mappings);
        let engine = ControllerEngine::open(engine_api::APP_DISPLAY_NAME, mappings_dir, shipped)
            .map_err(|e| e.to_string())?;
        let engine = Arc::new(Mutex::new(engine));
        let stop = Arc::new(AtomicBool::new(false));
        let sink: Arc<Mutex<Option<StreamSink<ControllerEvt>>>> = Arc::new(Mutex::new(None));
        let engine_buses = engine_buses.buses();
        let library_buses = library_buses.buses();

        let mut threads = Vec::with_capacity(3);
        {
            let stop_flag = Arc::clone(&stop);
            let controller_engine = Arc::clone(&engine);
            let sink = Arc::clone(&sink);
            spawn_named("controller-poll", &stop, &mut threads, move || loop {
                let events = if let Ok(mut eng) = controller_engine.lock() {
                    let _ = eng.poll_devices();
                    eng.take_events()
                } else {
                    Vec::new()
                };
                emit_events(&sink, events);
                let mut waited = Duration::ZERO;
                while waited < DEVICE_POLL_INTERVAL {
                    if stop_flag.load(Ordering::Relaxed) {
                        return;
                    }
                    std::thread::sleep(Duration::from_millis(100));
                    waited += Duration::from_millis(100);
                }
            })?;
        }
        {
            let stop_flag = Arc::clone(&stop);
            let controller_engine = Arc::clone(&engine);
            let sink = Arc::clone(&sink);
            let engine_buses = engine_buses.clone();
            let library_buses = library_buses.clone();
            spawn_named("controller-pump", &stop, &mut threads, move || {
                while !stop_flag.load(Ordering::Relaxed) {
                    let mut bus = HostPublish {
                        engine: engine_buses.clone(),
                        library: library_buses.clone(),
                    };
                    let events = if let Ok(mut eng) = controller_engine.lock() {
                        eng.pump(&mut bus);
                        eng.take_events()
                    } else {
                        Vec::new()
                    };
                    emit_events(&sink, events);
                    std::thread::sleep(PUMP_INTERVAL);
                }
            })?;
        }
        {
            let stop_flag = Arc::clone(&stop);
            let controller_engine = Arc::clone(&engine);
            let engine_buses = engine_buses.clone();
            let library_buses = library_buses.clone();
            spawn_named("controller-mirror", &stop, &mut threads, move || {
                mirror_engine_library_to_controller(
                    stop_flag,
                    controller_engine,
                    engine_buses,
                    library_buses,
                );
            })?;
        }

        Ok(Self {
            engine,
            stop,
            threads: Mutex::new(threads),
            sink,
        })
    }

    pub fn list_mappings(&self) -> Result<Vec<ControllerMappingInfo>, String> {
        let eng = self
            .engine
            .lock()
            .map_err(|_| "controller lock poisoned".to_string())?;
        eng.list_mappings()
            .map(|rows| rows.into_iter().map(mapping_info).collect())
            .map_err(|e| e.to_string())
    }

    pub fn list_devices(&self) -> Result<Vec<ControllerDeviceInfo>, String> {
        let mut eng = self
            .engine
            .lock()
            .map_err(|_| "controller lock poisoned".to_string())?;
        eng.list_devices()
            .map(|rows| rows.into_iter().map(device_info).collect())
            .map_err(|e| e.to_string())
    }

    pub fn pending_offers(&self) -> Result<Vec<ControllerEvt>, String> {
        let eng = self
            .engine
            .lock()
            .map_err(|_| "controller lock poisoned".to_string())?;
        Ok(eng.pending_offers().into_iter().map(map_evt).collect())
    }

    pub fn enable_mapping(
        &self,
        mapping_id: String,
        port_name: Option<String>,
    ) -> Result<(), String> {
        let mut eng = self
            .engine
            .lock()
            .map_err(|_| "controller lock poisoned".to_string())?;
        eng.enable_mapping(&mapping_id, port_name.as_deref())
            .map_err(|e| e.to_string())
    }

    pub fn disable_mapping(&self, mapping_id: String) -> Result<(), String> {
        let mut eng = self
            .engine
            .lock()
            .map_err(|_| "controller lock poisoned".to_string())?;
        eng.disable_mapping(&mapping_id).map_err(|e| e.to_string())
    }

    pub fn update_mapping(&self, mapping_id: String) -> Result<(), String> {
        let mut eng = self
            .engine
            .lock()
            .map_err(|_| "controller lock poisoned".to_string())?;
        eng.update_mapping(&mapping_id).map_err(|e| e.to_string())
    }

    pub fn update_all_mappings(&self) -> Result<(), String> {
        let mut eng = self
            .engine
            .lock()
            .map_err(|_| "controller lock poisoned".to_string())?;
        eng.update_all_mappings().map_err(|e| e.to_string())
    }

    /// Forward mapping offer/attach/detach events to Dart via FRB `StreamSink`.
    ///
    /// Replaces any previous sink so repeated subscribe calls do not leak.
    pub fn subscribe_events(&self, sink: StreamSink<ControllerEvt>) -> Result<(), String> {
        let mut slot = self
            .sink
            .lock()
            .map_err(|_| "controller sink poisoned".to_string())?;
        *slot = Some(sink);
        Ok(())
    }
}

fn emit_events(sink: &Mutex<Option<StreamSink<ControllerEvt>>>, events: Vec<ControllerEvent>) {
    if events.is_empty() {
        return;
    }
    let Ok(mut guard) = sink.lock() else {
        return;
    };
    let failed = {
        let Some(slot) = guard.as_ref() else {
            return;
        };
        events.into_iter().any(|ev| slot.add(map_evt(ev)).is_err())
    };
    if failed {
        *guard = None;
    }
}

fn mapping_info(info: MappingInfo) -> ControllerMappingInfo {
    ControllerMappingInfo {
        id: info.id,
        device_id: info.device_id,
        vendor_name: info.vendor_name,
        product_name: info.product_name,
        description: info.description,
        midi_name_contains: info.midi_name_contains,
        attached: info.attached,
    }
}

fn device_info(info: DeviceInfo) -> ControllerDeviceInfo {
    ControllerDeviceInfo {
        port_name: info.port_name,
        direction: match info.direction {
            DeviceDirection::Input => ControllerDeviceDirection::Input,
            DeviceDirection::Output => ControllerDeviceDirection::Output,
        },
        matched_mapping_id: info.matched_mapping_id,
    }
}

fn map_evt(ev: ControllerEvent) -> ControllerEvt {
    match ev {
        ControllerEvent::MappingOffer {
            mapping_id,
            device_name,
            port_name,
        } => ControllerEvt {
            kind: ControllerEvtKind::MappingOffer,
            mapping_id: Some(mapping_id),
            device_name: Some(device_name),
            port_name: Some(port_name),
        },
        ControllerEvent::MappingAttached {
            mapping_id,
            port_name,
        } => ControllerEvt {
            kind: ControllerEvtKind::MappingAttached,
            mapping_id: Some(mapping_id),
            device_name: None,
            port_name: Some(port_name),
        },
        ControllerEvent::MappingDetached { mapping_id } => ControllerEvt {
            kind: ControllerEvtKind::MappingDetached,
            mapping_id: Some(mapping_id),
            device_name: None,
            port_name: None,
        },
    }
}

fn stop_and_join(stop: &AtomicBool, threads: &mut Vec<JoinHandle<()>>) {
    stop.store(true, Ordering::Relaxed);
    for handle in threads.drain(..) {
        let _ = handle.join();
    }
}

fn spawn_named(
    name: &str,
    stop: &AtomicBool,
    threads: &mut Vec<JoinHandle<()>>,
    f: impl FnOnce() + Send + 'static,
) -> Result<(), String> {
    match std::thread::Builder::new().name(name.into()).spawn(f) {
        Ok(handle) => {
            threads.push(handle);
            Ok(())
        }
        Err(e) => {
            stop_and_join(stop, threads);
            Err(e.to_string())
        }
    }
}

fn hot_cue_slots_lib(cues: &[LibraryHotCue]) -> [Option<i32>; HOT_CUE_SLOT_COUNT] {
    let mut slots = [None; HOT_CUE_SLOT_COUNT];
    for cue in cues {
        let idx = cue.slot as usize;
        if idx < slots.len() {
            slots[idx] = Some(cue.position_ms);
        }
    }
    slots
}

fn hot_cue_slots_deck(cues: &[DeckHotCue]) -> [Option<i32>; HOT_CUE_SLOT_COUNT] {
    let mut slots = [None; HOT_CUE_SLOT_COUNT];
    for cue in cues {
        let idx = cue.slot as usize;
        if idx < slots.len() {
            slots[idx] = Some(cue.position_ms);
        }
    }
    slots
}

fn apply_pad_mode(eng: &Arc<Mutex<ControllerEngine>>, deck: u16, mode: PadMode) {
    let Ok(mut ctrl) = eng.lock() else {
        return;
    };
    ctrl.set_deck_pad_mode(deck, mode);
}

fn apply_hot_cues(
    eng: &Arc<Mutex<ControllerEngine>>,
    deck: u16,
    slots: [Option<i32>; HOT_CUE_SLOT_COUNT],
) {
    let Ok(mut ctrl) = eng.lock() else {
        return;
    };
    ctrl.set_deck_hot_cues(deck, slots);
}

fn mirror_engine_library_to_controller(
    stop: Arc<AtomicBool>,
    controller: Arc<Mutex<ControllerEngine>>,
    engine_buses: EngineBuses,
    library_buses: LibraryBuses,
) {
    let Ok(engine_rx) = engine_buses.subscribe_evt_all() else {
        return;
    };
    let Ok(library_rx) = library_buses.subscribe_evt_all() else {
        return;
    };
    let mut deck_tracks: [Option<String>; 4] = Default::default();
    while !stop.load(Ordering::Relaxed) {
        match engine_rx.recv_timeout(Duration::from_millis(5)) {
            Ok(Some(ev)) => apply_engine_mirror(&controller, &mut deck_tracks, ev.as_ref()),
            Ok(None) => {}
            Err(_) => return,
        }
        match library_rx.recv_timeout(Duration::from_millis(5)) {
            Ok(Some(ev)) => apply_library_mirror(&controller, &deck_tracks, ev.as_ref()),
            Ok(None) => {}
            Err(_) => return,
        }
    }
}

fn apply_engine_mirror(
    controller: &Arc<Mutex<ControllerEngine>>,
    deck_tracks: &mut [Option<String>; 4],
    ev: &engine_core::Evt,
) {
    let Ok(body) = decode_evt_body(ev.payload()) else {
        return;
    };
    match body {
        EvtBody::DeckUpdated {
            id,
            track_id,
            pad_mode,
            hot_cues,
            ..
        } => {
            let idx = (id as usize).min(3);
            deck_tracks[idx] = track_id;
            apply_pad_mode(controller, id, pad_mode);
            apply_hot_cues(controller, id, hot_cue_slots_deck(&hot_cues));
        }
        EvtBody::EngineStatus { status } => {
            for deck in status.decks {
                let idx = (deck.id as usize).min(3);
                deck_tracks[idx] = deck.track_id;
                apply_pad_mode(controller, deck.id, deck.pad_mode);
                apply_hot_cues(controller, deck.id, hot_cue_slots_deck(&deck.hot_cues));
            }
        }
        _ => {}
    }
}

fn apply_library_mirror(
    controller: &Arc<Mutex<ControllerEngine>>,
    deck_tracks: &[Option<String>; 4],
    ev: &library::Evt,
) {
    let Ok(body) = decode_library_evt(ev.payload()) else {
        return;
    };
    let LibraryEvtBody::HotCuesChanged { track_id, hot_cues } = body else {
        return;
    };
    let slots = hot_cue_slots_lib(&hot_cues);
    for (i, loaded) in deck_tracks.iter().enumerate() {
        if loaded.as_deref() == Some(track_id.as_str()) {
            apply_hot_cues(controller, i as u16, slots);
        }
    }
}

/// ponytail: compile-time walk to repo `mappings/` — Flutter release builds need
/// bundled resources (Tauri uses `resource_dir` first). Override via `shipped_mappings_dir`.
fn resolve_shipped_mappings() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for _ in 0..6 {
        let candidate = dir.join("mappings");
        if candidate.is_dir() {
            return candidate;
        }
        if !dir.pop() {
            break;
        }
    }
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mappings")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stop_and_join_stops_spawned_thread() {
        let stop = Arc::new(AtomicBool::new(false));
        let mut threads = Vec::new();
        let watch = Arc::clone(&stop);
        spawn_named("controller-test", stop.as_ref(), &mut threads, move || {
            while !watch.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(1));
            }
        })
        .unwrap();
        assert_eq!(threads.len(), 1);
        stop_and_join(stop.as_ref(), &mut threads);
        assert!(threads.is_empty());
    }
}
