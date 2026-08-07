//! Mapping runtime: MIDI in → cmds; snapshot → MIDI out.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use engine_api::{CmdBody, Kind, Origin, PadMode};
use library_api::{EvtBody as LibraryEvtBody, Kind as LibraryKind, Origin as LibraryOrigin};

use crate::action::{resolve_action, ControlSnapshot, RoutedAction};
use crate::action_id::{bind_origin, parse_action_id, BoundOrigin};
use crate::bundle::MappingBundle;
use crate::device::SECTION_CUSTOM;
use crate::error::{LoadError, MidiPortError, RuntimeError};
use crate::map_file::{InputBinding, OutputTarget};
use crate::midi::{norm_from_cc14, parse_short, MidiEndpoint, ShortMsg};
use crate::script::{ScriptHost, ScriptRuntime};

pub const SOFT_TAKEOVER_THRESHOLD: f32 = 3.0 / 127.0;
const CC_COALESCE: Duration = Duration::from_nanos(1_000_000_000 / 60);
/// Script `idle_heartbeat` cadence when no deck is playing.
const IDLE_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(1);

/// Snapshot / LED array slot for a deck origin (defense if an OOR index slips past load).
fn deck_slot(d: u16) -> usize {
    (d as usize).min(3)
}

/// Latest absolute CC waiting for ≤60 Hz flush.
#[derive(Clone, Debug)]
struct PendingCc {
    section: String,
    alias: String,
    norm: f32,
    active: bool,
}

pub trait ActionPublish {
    fn publish_engine(&mut self, origin: Origin, kind: Kind, body: CmdBody);
    fn publish_library(&mut self, origin: LibraryOrigin, kind: LibraryKind, body: LibraryEvtBody);
}

/// Compatibility alias for engine-only hosts/tests.
pub trait BusPublish: ActionPublish {
    fn publish(&mut self, origin: Origin, kind: Kind, body: CmdBody) {
        self.publish_engine(origin, kind, body);
    }
}

pub trait MidiOut {
    fn send(&mut self, bytes: &[u8]);
}

/// Host MIDI port (output). Input is pushed via [`MappingSession::handle_midi`].
pub trait MidiPort {
    fn send(&mut self, bytes: &[u8]) -> Result<(), MidiPortError>;
}

impl<T: MidiOut + ?Sized> MidiOut for &mut T {
    fn send(&mut self, bytes: &[u8]) {
        (**self).send(bytes);
    }
}

pub struct MappingSession {
    pub bundle: MappingBundle,
    snapshot: ControlSnapshot,
    /// Active custom.* modifiers (held).
    modifiers: HashSet<String>,
    /// Soft-takeover latched keys: "section.alias".
    soft_latched: HashSet<String>,
    /// Last CC publish time per "section.alias".
    cc_last: HashMap<String, Instant>,
    /// Latest CC not yet published (rate-limited); flushed by [`Self::flush_coalesced`].
    cc_pending: HashMap<String, PendingCc>,
    /// 14-bit CC pair state: "section.alias" → (msb, lsb).
    cc14_state: HashMap<String, (Option<u8>, Option<u8>)>,
    /// Last edge state for notes: "section.alias" → active.
    note_state: HashMap<String, bool>,
    /// Output signal cache: "section.alias" → active.
    output_state: HashMap<String, bool>,
    /// Last VU MIDI data2 per "section.vu_meter" (skip duplicates).
    vu_out: HashMap<String, u8>,
    script: Option<ScriptRuntime>,
    /// Last script `idle_heartbeat`; `None` → first call fires immediately.
    last_idle_heartbeat: Option<Instant>,
}

impl MappingSession {
    pub fn from_bundle(bundle: MappingBundle) -> Result<Self, LoadError> {
        let script = match &bundle.script_source {
            Some(src) => Some(ScriptRuntime::compile(src)?),
            None => None,
        };
        Ok(Self {
            bundle,
            snapshot: ControlSnapshot::default(),
            modifiers: HashSet::new(),
            soft_latched: HashSet::new(),
            cc_last: HashMap::new(),
            cc_pending: HashMap::new(),
            cc14_state: HashMap::new(),
            note_state: HashMap::new(),
            output_state: HashMap::new(),
            vu_out: HashMap::new(),
            script,
            last_idle_heartbeat: None,
        })
    }

    pub fn snapshot(&self) -> &ControlSnapshot {
        &self.snapshot
    }

    /// Hot-cue pad routing uses these positions; keep in sync with library `HotCuesChanged`.
    pub fn set_deck_hot_cues(
        &mut self,
        deck: u16,
        cues: [Option<i32>; 8],
        midi: &mut impl MidiOut,
    ) {
        let i = (deck as usize).min(3);
        self.snapshot.hot_cues[i] = cues;
        self.refresh_hot_cue_leds(deck, midi);
    }

    /// Re-send pad LED MIDI for the deck's hot-cue slots (also after pad-mode changes).
    pub fn refresh_hot_cue_leds(&mut self, deck: u16, midi: &mut impl MidiOut) {
        let i = (deck as usize).min(3);
        let cues = self.snapshot.hot_cues[i];
        let section = format!("deck_{}", deck + 1);
        for (slot, pos) in cues.iter().enumerate() {
            let alias = format!("hot_cue_{}", slot + 1);
            // Force re-send: HW often clears pad LEDs on mode switch.
            self.output_state.remove(&format!("{section}.{alias}"));
            self.apply_output_signal(&section, &alias, pos.is_some(), midi);
        }
    }

    pub fn set_control_value(&mut self, origin: Origin, key: &str, value: f32) {
        self.snapshot.set_value(origin, key, value);
        // Engine moved under the hand → drop latch so soft-takeover re-engages.
        if matches!(
            key,
            "volume"
                | "filter"
                | "filter_db"
                | "gain"
                | "gain_db"
                | "speed"
                | "tempo"
                | "eq_low"
                | "eq_mid"
                | "eq_high"
                | "crossfader"
                | "cue_mix"
        ) {
            self.soft_latched
                .retain(|k| !k.ends_with(&format!(".{key}")) && !k.contains(key));
            // Clear all latches for absolute controls on this origin — simple & correct.
            self.soft_latched.clear();
        }
    }

    pub fn on_init(
        &mut self,
        bus: &mut impl ActionPublish,
        midi: &mut impl MidiOut,
    ) -> Result<(), RuntimeError> {
        self.run_lifecycle("on_init", bus, midi)
    }

    pub fn on_shutdown(
        &mut self,
        bus: &mut impl ActionPublish,
        midi: &mut impl MidiOut,
    ) -> Result<(), RuntimeError> {
        self.run_lifecycle("on_shutdown", bus, midi)
    }

    /// Drive continuous `vu_meter` CC out (Mixxx scale: level×150, clamp 127).
    pub fn set_deck_vu(&mut self, deck: u16, level: f32, midi: &mut impl MidiOut) {
        let section = format!("deck_{}", deck.min(3) + 1);
        let Some(ep) = self.bundle.device.endpoint(&section, "vu_meter") else {
            return;
        };
        let value = (level.clamp(0.0, 1.0) * 150.0).min(127.0).round() as u8;
        let key = format!("{section}.vu_meter");
        if self.vu_out.get(&key).copied() == Some(value) {
            return;
        }
        self.vu_out.insert(key, value);
        midi.send(&ep.to_bytes(Some(value)));
    }

    /// Optional script keepalive while all decks are stopped. Call from the MIDI pump.
    pub fn idle_heartbeat(
        &mut self,
        bus: &mut impl ActionPublish,
        midi: &mut impl MidiOut,
    ) -> Result<(), RuntimeError> {
        if self.bundle.map.lifecycle.fn_for("idle_heartbeat").is_none() {
            return Ok(());
        }
        if self.script.is_none() {
            return Ok(());
        }
        if self.snapshot.playing.iter().any(|&p| p) {
            return Ok(());
        }
        if self
            .last_idle_heartbeat
            .is_some_and(|t| t.elapsed() < IDLE_HEARTBEAT_INTERVAL)
        {
            return Ok(());
        }
        self.last_idle_heartbeat = Some(Instant::now());
        self.run_lifecycle("idle_heartbeat", bus, midi)
    }

    fn run_lifecycle(
        &mut self,
        event: &str,
        bus: &mut impl ActionPublish,
        midi: &mut impl MidiOut,
    ) -> Result<(), RuntimeError> {
        let Some(fn_name) = self
            .bundle
            .map
            .lifecycle
            .fn_for(event)
            .map(str::to_string)
        else {
            return Ok(());
        };
        let Some(script) = self.script.as_mut() else {
            return Ok(());
        };
        let mut host = ScriptHost {
            bus,
            midi,
            snapshot: &self.snapshot,
            modifiers: &self.modifiers,
        };
        script.call_hook(&fn_name, &mut host)
    }

    pub fn handle_midi(
        &mut self,
        bytes: &[u8],
        bus: &mut impl ActionPublish,
        midi: &mut impl MidiOut,
    ) {
        let Some(parsed) = parse_short(bytes) else {
            return;
        };
        let Some((section, alias, ep)) = self.bundle.device.find_input_match(parsed.match_key())
        else {
            return;
        };
        // Clone endpoint fields we need after borrow ends.
        let is_cc14 = ep.is_cc14();
        let cc14_pair = ep.cc14_pair();
        let section = section.to_string();
        let alias = alias.to_string();
        let key = format!("{section}.{alias}");

        if section == SECTION_CUSTOM {
            let mod_key = format!("custom.{alias}");
            if parsed.active() {
                self.modifiers.insert(mod_key);
            } else {
                self.modifiers.remove(&mod_key);
            }
            // custom is not declarative-input bindable; still allow script-only later
            return;
        }

        // Resolve 0..1 value (cc14 pairs MSB+LSB).
        let mut value_01 = parsed.value_01();
        let is_cc = matches!(parsed.msg, ShortMsg::Cc { .. });
        if is_cc14 {
            let ShortMsg::Cc { cc, value, .. } = parsed.msg else {
                return;
            };
            let Some((msb_cc, lsb_cc)) = cc14_pair else {
                return;
            };
            let entry = self.cc14_state.entry(key.clone()).or_insert((None, None));
            if cc == msb_cc {
                entry.0 = Some(value);
            } else if cc == lsb_cc {
                entry.1 = Some(value);
            } else {
                return;
            }
            let (Some(msb), Some(lsb)) = *entry else {
                return; // wait until both bytes seen
            };
            value_01 = norm_from_cc14(msb, lsb);
        }

        // Edge for notes: only process transitions for button-like msgs.
        if !is_cc {
            let prev = self.note_state.get(&key).copied().unwrap_or(false);
            if prev == parsed.active() {
                return;
            }
            self.note_state.insert(key.clone(), parsed.active());
            self.dispatch_input(
                &section,
                &alias,
                &key,
                value_01,
                parsed.active(),
                false,
                bus,
                midi,
            );
            return;
        }

        // Absolute CCs: keep latest value; publish at ≤60 Hz (flush covers the final move).
        // Relative library browse must not coalesce — each tick is a discrete row step.
        if self.is_relative_library_nav(&section, &alias) {
            self.dispatch_input(
                &section,
                &alias,
                &key,
                value_01,
                parsed.active(),
                false,
                bus,
                midi,
            );
            return;
        }
        self.cc_pending.insert(
            key.clone(),
            PendingCc {
                section: section.clone(),
                alias: alias.clone(),
                norm: value_01,
                active: parsed.active(),
            },
        );
        let now = Instant::now();
        if let Some(last) = self.cc_last.get(&key) {
            if now.duration_since(*last) < CC_COALESCE {
                return;
            }
        }
        self.flush_pending_key(&key, bus, midi);
    }

    /// Relative select-knob bindings (e.g. `LibraryNavigation::navigate`) skip CC coalesce.
    fn is_relative_library_nav(&self, section: &str, alias: &str) -> bool {
        self.bundle
            .map
            .bindings_for(section, alias)
            .iter()
            .any(|b| b.action.as_deref() == Some("LibraryNavigation::navigate"))
    }

    /// Publish any rate-limited CCs whose coalesce window has elapsed (call from MIDI pump).
    pub fn flush_coalesced(&mut self, bus: &mut impl ActionPublish, midi: &mut impl MidiOut) {
        let now = Instant::now();
        let ready: Vec<String> = self
            .cc_pending
            .keys()
            .filter(|key| {
                self.cc_last
                    .get(*key)
                    .map(|last| now.duration_since(*last) >= CC_COALESCE)
                    .unwrap_or(true)
            })
            .cloned()
            .collect();
        for key in ready {
            self.flush_pending_key(&key, bus, midi);
        }
    }

    fn flush_pending_key(
        &mut self,
        key: &str,
        bus: &mut impl ActionPublish,
        midi: &mut impl MidiOut,
    ) {
        let Some(pending) = self.cc_pending.get(key).cloned() else {
            return;
        };
        if self.dispatch_input(
            &pending.section,
            &pending.alias,
            key,
            pending.norm,
            pending.active,
            true,
            bus,
            midi,
        ) {
            self.cc_pending.remove(key);
        }
    }

    /// Resolve binding → soft-takeover → publish. Returns true if a publish was sent.
    fn dispatch_input(
        &mut self,
        section: &str,
        alias: &str,
        key: &str,
        norm: f32,
        active: bool,
        is_cc: bool,
        bus: &mut impl ActionPublish,
        midi: &mut impl MidiOut,
    ) -> bool {
        let bindings = self.bundle.map.bindings_for(section, alias);
        if bindings.is_empty() {
            return false;
        }
        let binding = select_binding(&bindings, &self.modifiers);
        let Some(binding) = binding else {
            return false;
        };

        if let Some(script_fn) = &binding.script {
            if let Some(script) = self.script.as_mut() {
                // Script bindings get a null midi sink here; host can call on_init with midi.
                struct NullMidi;
                impl MidiOut for NullMidi {
                    fn send(&mut self, _bytes: &[u8]) {}
                }
                let mut null = NullMidi;
                let mut host = ScriptHost {
                    bus,
                    midi: &mut null,
                    snapshot: &self.snapshot,
                    modifiers: &self.modifiers,
                };
                let _ = script.call_named(script_fn, &mut host, norm, active);
            }
            if is_cc {
                self.cc_last.insert(key.to_string(), Instant::now());
            }
            return true;
        }

        let action = match &binding.action {
            Some(a) => a.as_str(),
            None => return false,
        };
        let Ok((template, leaf)) = parse_action_id(action) else {
            return false;
        };
        let Ok(bound) = bind_origin(template, section) else {
            return false;
        };

        if binding.soft_takeover_effective() {
            if let BoundOrigin::Engine(origin) = &bound {
                if let Some(engine_v) = self.snapshot.get_norm_for_action(origin.clone(), leaf) {
                    let latched = self.soft_latched.contains(key);
                    if !latched {
                        let dist = (norm - engine_v).abs();
                        if dist > SOFT_TAKEOVER_THRESHOLD {
                            return false;
                        }
                        self.soft_latched.insert(key.to_string());
                    }
                }
            }
        }

        let Some(routed) = resolve_action(action, section, norm, active, &self.snapshot) else {
            return false;
        };

        if is_cc {
            self.cc_last.insert(key.to_string(), Instant::now());
        }
        match &routed {
            RoutedAction::EngineCmd {
                origin: o,
                kind,
                body,
            } => {
                // Mirror absolute values into snapshot after publish intent
                match body {
                    CmdBody::SetVolume { volume } => {
                        if let Origin::Deck(d) = *o {
                            self.snapshot.volume[deck_slot(d)] = *volume;
                        }
                    }
                    CmdBody::SetFilter { filter_db } => {
                        if let Origin::Deck(d) = *o {
                            self.snapshot.filter_db[deck_slot(d)] = *filter_db;
                        }
                    }
                    CmdBody::SetGainTrim { gain_db } => {
                        if let Origin::Deck(d) = *o {
                            self.snapshot.gain_db[deck_slot(d)] = *gain_db;
                        }
                    }
                    CmdBody::SetSpeed { speed } => {
                        if let Origin::Deck(d) = *o {
                            self.snapshot.speed[deck_slot(d)] = *speed;
                        }
                    }
                    CmdBody::SetEq { low, mid, high } => {
                        if let Origin::Deck(d) = *o {
                            let i = deck_slot(d);
                            self.snapshot.eq_low[i] = *low;
                            self.snapshot.eq_mid[i] = *mid;
                            self.snapshot.eq_high[i] = *high;
                        }
                    }
                    CmdBody::SetCrossfader { position } => {
                        self.snapshot.crossfader = *position;
                    }
                    CmdBody::SetCueMix { mix } => {
                        self.snapshot.cue_mix = *mix;
                    }
                    CmdBody::SetHeadphoneCue { enabled } => {
                        if let Origin::Deck(d) = *o {
                            let i = deck_slot(d);
                            self.snapshot.headphone_cue[i] = *enabled;
                            let deck_section = format!("deck_{}", d + 1);
                            self.apply_output_signal(
                                &deck_section,
                                "headphone_cue",
                                *enabled,
                                midi,
                            );
                        }
                    }
                    CmdBody::SetMasterCue { enabled } => {
                        self.snapshot.master_cue = *enabled;
                    }
                    CmdBody::SetPadMode { mode } => {
                        if let Origin::Deck(d) = *o {
                            let i = deck_slot(d);
                            self.snapshot.pad_mode[i] = *mode;
                            if *mode == PadMode::HotCue {
                                self.refresh_hot_cue_leds(d, midi);
                            }
                        }
                    }
                    _ => {}
                }
                if matches!(kind, Kind::Play | Kind::TriggerHotCue) {
                    if let Origin::Deck(d) = *o {
                        let i = deck_slot(d);
                        self.snapshot.playing[i] = true;
                        let deck_section = format!("deck_{}", d + 1);
                        self.apply_output_signal(&deck_section, "play_pause", true, midi);
                    }
                }
                if matches!(kind, Kind::Pause) {
                    if let Origin::Deck(d) = *o {
                        let i = deck_slot(d);
                        self.snapshot.playing[i] = false;
                        let deck_section = format!("deck_{}", d + 1);
                        self.apply_output_signal(&deck_section, "play_pause", false, midi);
                    }
                }
                bus.publish_engine(o.clone(), kind.clone(), body.clone());
            }
            RoutedAction::LibraryEvt { origin, kind, body } => {
                bus.publish_library(origin.clone(), kind.clone(), body.clone());
            }
        }
        true
    }

    /// Update playing signal and emit mapped LED MIDI if changed.
    pub fn on_deck_playing(&mut self, deck: u16, playing: bool, midi: &mut impl MidiOut) {
        let i = (deck as usize).min(3);
        self.snapshot.playing[i] = playing;
        let section = format!("deck_{}", deck + 1);
        self.apply_output_signal(&section, "play_pause", playing, midi);
    }

    pub fn apply_output_signal(
        &mut self,
        section: &str,
        alias: &str,
        active: bool,
        midi: &mut impl MidiOut,
    ) {
        let key = format!("{section}.{alias}");
        if self.output_state.get(&key).copied() == Some(active) {
            return;
        }
        self.output_state.insert(key, active);
        let Some(sec) = self.bundle.map.outputs.get(section) else {
            return;
        };
        let Some(out) = sec.get(alias) else {
            return;
        };
        let target = if active { &out.on } else { &out.off };
        if let Some(bytes) = resolve_output_bytes(&self.bundle.device, section, target) {
            midi.send(&bytes);
        }
    }
}

fn select_binding<'a>(
    bindings: &'a [InputBinding],
    modifiers: &HashSet<String>,
) -> Option<&'a InputBinding> {
    // Prefer first binding whose modifier is active.
    for b in bindings {
        if let Some(m) = &b.modifier {
            if modifiers.contains(m) {
                return Some(b);
            }
        }
    }
    // Else first unmodified binding.
    bindings.iter().find(|b| b.modifier.is_none())
}

fn resolve_output_bytes(
    device: &crate::device::DeviceFile,
    section: &str,
    target: &OutputTarget,
) -> Option<[u8; 3]> {
    match target {
        OutputTarget::Inline(ep) => Some(ep.to_bytes(None)),
        OutputTarget::Alias(name) => {
            let ep = if name.contains('.') {
                let (_, _, ep) = device.resolve_ref(name)?;
                ep
            } else if let Some(ep) = device.endpoint(section, name) {
                ep
            } else {
                // Search all sections
                device
                    .sections
                    .values()
                    .find_map(|m| m.get(name.as_str()))?
            };
            Some(output_endpoint_bytes(ep))
        }
    }
}

fn output_endpoint_bytes(ep: &MidiEndpoint) -> [u8; 3] {
    ep.to_bytes(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    struct CaptureBus {
        cmds: Vec<(Origin, Kind, CmdBody)>,
    }

    impl ActionPublish for CaptureBus {
        fn publish_engine(&mut self, origin: Origin, kind: Kind, body: CmdBody) {
            self.cmds.push((origin, kind, body));
        }
        fn publish_library(
            &mut self,
            _origin: LibraryOrigin,
            _kind: LibraryKind,
            _body: LibraryEvtBody,
        ) {
        }
    }

    struct NullMidi;
    impl MidiOut for NullMidi {
        fn send(&mut self, _bytes: &[u8]) {}
    }

    fn session() -> MappingSession {
        let b = crate::load_bundle(Path::new("tests/fixtures/valid-minimal")).unwrap();
        MappingSession::from_bundle(b).unwrap()
    }

    #[test]
    fn soft_threshold_is_3_of_127() {
        assert!((SOFT_TAKEOVER_THRESHOLD - 3.0 / 127.0).abs() < 1e-9);
    }

    #[test]
    fn cc_coalesce_keeps_latest_until_flush() {
        let mut s = session();
        // Disable soft-takeover distance: engine volume starts at 1.0; latch by matching first.
        s.set_control_value(Origin::Deck(0), "volume", 0.0);
        let mut bus = CaptureBus { cmds: vec![] };

        // First CC publishes (0 → latch + set).
        s.handle_midi(&[0xB0, 0x13, 0], &mut bus, &mut NullMidi);
        assert_eq!(bus.cmds.len(), 1);

        // Burst within coalesce window — only pending, no extra publishes.
        s.handle_midi(&[0xB0, 0x13, 32], &mut bus, &mut NullMidi);
        s.handle_midi(&[0xB0, 0x13, 96], &mut bus, &mut NullMidi);
        s.handle_midi(&[0xB0, 0x13, 127], &mut bus, &mut NullMidi);
        assert_eq!(
            bus.cmds.len(),
            1,
            "rate limit must not publish intermediates"
        );

        // Window elapsed → flush publishes the latest (127).
        if let Some(t) = s.cc_last.get_mut("deck_1.volume") {
            *t = Instant::now() - CC_COALESCE - Duration::from_millis(1);
        }
        s.flush_coalesced(&mut bus, &mut NullMidi);
        assert_eq!(bus.cmds.len(), 2);
        match &bus.cmds[1].2 {
            CmdBody::SetVolume { volume } => {
                assert!((*volume - 1.0).abs() < 1e-5, "volume={volume}");
            }
            other => panic!("expected SetVolume, got {other:?}"),
        }
    }
}
