//! Mapping runtime: MIDI in → cmds; snapshot → MIDI out.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use engine_api::{CmdBody, Kind, Origin};
use library_api::{EvtBody as LibraryEvtBody, Kind as LibraryKind, Origin as LibraryOrigin};

use crate::action::{resolve_action, ControlSnapshot, ControlValue, DeckFeedback, RoutedAction};
use crate::bundle::MappingBundle;
use crate::device::SECTION_CUSTOM;
use crate::error::{LoadError, MidiPortError, RuntimeError};
use crate::map_file::{InputBinding, OutputTarget};
use crate::midi::{decode_relative, norm_from_cc14, parse_short, MidiEndpoint, ShortMsg};
use crate::script::{ScriptHost, ScriptRuntime};

const CC_COALESCE: Duration = Duration::from_nanos(1_000_000_000 / 60);
/// Script `idle_heartbeat` cadence when no deck is playing.
const IDLE_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(1);
/// Half period of a blinking LED (on → off → on).
pub const BLINK_HALF_PERIOD: Duration = Duration::from_millis(200);

/// LED state resolved from a `map.toml` output `signal`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LedState {
    #[default]
    Off,
    On,
    /// Signal true and the binding declares a `blink` target; the session
    /// alternates `blink` / `off` on [`BLINK_HALF_PERIOD`].
    Blink,
}

/// Snapshot / LED array slot for a deck origin (defense if an OOR index slips past load).
fn deck_slot(d: u16) -> usize {
    (d as usize).min(3)
}

/// Rate-limited CC waiting for ≤60 Hz flush: absolute keeps latest; relative sums ticks.
#[derive(Clone, Debug)]
struct PendingCc {
    section: String,
    alias: String,
    value: ControlValue,
    active: bool,
}

/// Script-backed input binding failed; host logs with attach identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptBindingFailure {
    pub section: String,
    pub alias: String,
    pub script_fn: String,
    pub error: String,
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
    /// Per-deck playing (idempotent ++/-- for idle heartbeat).
    playing_decks: [bool; 4],
    /// Last CC publish time per "section.alias".
    cc_last: HashMap<String, Instant>,
    /// Latest CC not yet published (rate-limited); flushed by [`Self::flush_coalesced`].
    cc_pending: HashMap<String, PendingCc>,
    /// 14-bit CC pair state: "section.alias" → (msb, lsb).
    cc14_state: HashMap<String, (Option<u8>, Option<u8>)>,
    /// Last edge state for notes: "section.alias" → active.
    note_state: HashMap<String, bool>,
    /// LED cache: "section.alias" → last resolved state.
    output_state: HashMap<String, LedState>,
    /// Any output declares a `blink` target (skips the per-pump blink scan).
    has_blink: bool,
    /// Current blink phase (true = lit) and last phase-flip time.
    blink_on: bool,
    last_blink: Option<Instant>,
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
            has_blink: bundle
                .map
                .outputs
                .values()
                .flat_map(|s| s.values())
                .any(|o| o.blink.is_some()),
            bundle,
            snapshot: ControlSnapshot::default(),
            modifiers: HashSet::new(),
            playing_decks: [false; 4],
            cc_last: HashMap::new(),
            cc_pending: HashMap::new(),
            cc14_state: HashMap::new(),
            note_state: HashMap::new(),
            output_state: HashMap::new(),
            blink_on: false,
            last_blink: None,
            vu_out: HashMap::new(),
            script,
            last_idle_heartbeat: None,
        })
    }

    pub fn snapshot(&self) -> &ControlSnapshot {
        &self.snapshot
    }

    /// Engine deck state mirrored for LED feedback (one refresh per event).
    pub fn set_deck_feedback(&mut self, deck: u16, fb: &DeckFeedback, midi: &mut impl MidiOut) {
        let i = deck_slot(deck);
        self.playing_decks[i] = fb.playing;
        self.snapshot.playing[i] = fb.playing;
        self.snapshot.sync[i] = fb.sync;
        self.snapshot.quantize[i] = fb.quantize;
        self.snapshot.headphone_cue[i] = fb.headphone_cue;
        self.snapshot.loop_active[i] = fb.loop_active;
        self.snapshot.track_loaded[i] = fb.track_loaded;
        self.snapshot.loop_slots[i] = fb.loop_slots;
        self.snapshot.pad_mode[i] = fb.pad_mode;
        self.set_deck_hot_cues(deck, fb.hot_cues, midi);
    }

    /// LED/toggle-pause and the `trigger_hot_cue` shortcut use these positions.
    /// MIDI `pad n` publishes named press/release; it does not look up cue ms.
    pub fn set_deck_hot_cues(
        &mut self,
        deck: u16,
        cues: [Option<i32>; crate::HOT_CUE_SLOT_COUNT],
        midi: &mut impl MidiOut,
    ) {
        let i = deck_slot(deck);
        self.snapshot.hot_cues[i] = cues;
        self.refresh_deck_leds(deck, midi);
    }

    /// Mirror engine playhead so `loop_in` / `loop_out` can stamp `position_ms`.
    pub fn set_deck_position_ms(&mut self, deck: u16, position_ms: i32) {
        let i = deck_slot(deck);
        self.snapshot.position_ms[i] = position_ms;
    }

    /// Mirror mixer state (MASTER CUE LED).
    pub fn set_master_cue(&mut self, enabled: bool, midi: &mut impl MidiOut) {
        self.snapshot.master_cue = enabled;
        self.refresh_leds(crate::device::SECTION_MASTER, midi);
    }

    /// State-only mirror; use the `set_deck_*` setters to also refresh LEDs.
    pub fn set_control_value(&mut self, origin: Origin, key: &str, value: f32) {
        if key == "playing" {
            if let Origin::Deck(d) = origin {
                self.set_playing_deck(d, value > 0.5);
                return;
            }
        }
        self.snapshot.set_value(origin, key, value);
    }

    fn set_playing_deck(&mut self, deck: u16, playing: bool) {
        let i = deck_slot(deck);
        self.playing_decks[i] = playing;
        self.snapshot.playing[i] = playing;
    }

    /// Lifecycle init, then a forced LED sweep so attach / reconnect re-asserts
    /// every lamp (the unit may still hold state from a previous run).
    pub fn on_init(
        &mut self,
        bus: &mut impl ActionPublish,
        midi: &mut impl MidiOut,
    ) -> Result<(), RuntimeError> {
        self.run_lifecycle("on_init", bus, midi)?;
        self.refresh_all_leds(midi);
        Ok(())
    }

    /// Lifecycle shutdown, then explicitly clear every lamp so nothing stays lit.
    pub fn on_shutdown(
        &mut self,
        bus: &mut impl ActionPublish,
        midi: &mut impl MidiOut,
    ) -> Result<(), RuntimeError> {
        let result = self.run_lifecycle("on_shutdown", bus, midi);
        self.clear_leds(midi);
        result
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
        if self.playing_decks.iter().any(|&p| p) {
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
        let Some(fn_name) = self.bundle.map.lifecycle.fn_for(event).map(str::to_string) else {
            return Ok(());
        };
        let Some(script) = self.script.as_mut() else {
            return Ok(());
        };
        let mut host = ScriptHost {
            bus,
            midi,
            modifiers: &self.modifiers,
        };
        script.call_hook(&fn_name, &mut host)
    }

    pub fn handle_midi(
        &mut self,
        bytes: &[u8],
        bus: &mut impl ActionPublish,
        midi: &mut impl MidiOut,
    ) -> Option<ScriptBindingFailure> {
        let parsed = parse_short(bytes)?;
        let (section, alias, ep) = self.bundle.device.find_input_match(parsed.match_key())?;
        // Clone endpoint fields we need after borrow ends.
        let is_cc14 = ep.is_cc14();
        let cc14_pair = ep.cc14_pair();
        let relative_mode = ep.relative;
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
            // Modifiers gate the +SHIFT indicators, so re-resolve every section.
            self.refresh_all_sections(midi);
            // custom is not declarative-input bindable; still allow script-only later
            return None;
        }

        // Resolve absolute 0..1 (cc14 pairs MSB+LSB) or relative tick delta.
        let mut value = ControlValue::Absolute(parsed.value_01());
        let is_cc = matches!(parsed.msg, ShortMsg::Cc { .. });
        if is_cc14 {
            let ShortMsg::Cc { cc, value: raw, .. } = parsed.msg else {
                return None;
            };
            let (msb_cc, lsb_cc) = cc14_pair?;
            let entry = self.cc14_state.entry(key.clone()).or_insert((None, None));
            if cc == msb_cc {
                entry.0 = Some(raw);
            } else if cc == lsb_cc {
                entry.1 = Some(raw);
            } else {
                return None;
            }
            let (Some(msb), Some(lsb)) = *entry else {
                return None; // wait until both bytes seen
            };
            value = ControlValue::Absolute(norm_from_cc14(msb, lsb));
        } else if is_cc {
            if let Some(mode) = relative_mode {
                let ShortMsg::Cc { value: raw, .. } = parsed.msg else {
                    return None;
                };
                let delta = decode_relative(mode, raw);
                if delta == 0 {
                    return None;
                }
                value = ControlValue::Relative(delta);
            }
        }

        // Edge for notes: only process transitions for button-like msgs.
        if !is_cc {
            let prev = self.note_state.get(&key).copied().unwrap_or(false);
            if prev == parsed.active() {
                return None;
            }
            self.note_state.insert(key.clone(), parsed.active());
            let (_handled, fail) = self.dispatch_input(
                &section,
                &alias,
                &key,
                value,
                parsed.active(),
                false,
                bus,
                midi,
            );
            return fail;
        }

        // Absolute: keep latest. Relative: sum deltas. Both publish at ≤60 Hz.
        match value {
            ControlValue::Relative(delta) => match self.cc_pending.get_mut(&key) {
                Some(PendingCc {
                    value: ControlValue::Relative(sum),
                    ..
                }) => {
                    *sum = sum.saturating_add(delta);
                }
                _ => {
                    self.cc_pending.insert(
                        key.clone(),
                        PendingCc {
                            section: section.clone(),
                            alias: alias.clone(),
                            value: ControlValue::Relative(delta),
                            active: parsed.active(),
                        },
                    );
                }
            },
            ControlValue::Absolute(norm) => {
                self.cc_pending.insert(
                    key.clone(),
                    PendingCc {
                        section: section.clone(),
                        alias: alias.clone(),
                        value: ControlValue::Absolute(norm),
                        active: parsed.active(),
                    },
                );
            }
        }
        let now = Instant::now();
        if let Some(last) = self.cc_last.get(&key) {
            if now.duration_since(*last) < CC_COALESCE {
                return None;
            }
        }
        self.flush_pending_key(&key, bus, midi)
    }

    /// Publish any rate-limited CCs whose coalesce window has elapsed (call from MIDI pump).
    pub fn flush_coalesced(
        &mut self,
        bus: &mut impl ActionPublish,
        midi: &mut impl MidiOut,
    ) -> Vec<ScriptBindingFailure> {
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
        let mut fails = Vec::new();
        for key in ready {
            if let Some(fail) = self.flush_pending_key(&key, bus, midi) {
                fails.push(fail);
            }
        }
        fails
    }

    fn flush_pending_key(
        &mut self,
        key: &str,
        bus: &mut impl ActionPublish,
        midi: &mut impl MidiOut,
    ) -> Option<ScriptBindingFailure> {
        let pending = self.cc_pending.get(key).cloned()?;
        if matches!(pending.value, ControlValue::Relative(0)) {
            self.cc_pending.remove(key);
            return None;
        }
        let (handled, fail) = self.dispatch_input(
            &pending.section,
            &pending.alias,
            key,
            pending.value,
            pending.active,
            true,
            bus,
            midi,
        );
        if handled {
            self.cc_pending.remove(key);
        }
        fail
    }

    /// Resolve binding → soft-takeover → publish. Returns (handled, script failure).
    // ponytail: internal MIDI dispatch; packing into a struct is noise for CI threshold (7).
    #[allow(clippy::too_many_arguments)]
    fn dispatch_input(
        &mut self,
        section: &str,
        alias: &str,
        key: &str,
        value: ControlValue,
        active: bool,
        is_cc: bool,
        bus: &mut impl ActionPublish,
        midi: &mut impl MidiOut,
    ) -> (bool, Option<ScriptBindingFailure>) {
        let bindings = self.bundle.map.bindings_for(section, alias);
        if bindings.is_empty() {
            return (false, None);
        }
        let binding = select_binding(&bindings, &self.modifiers);
        let Some(binding) = binding else {
            return (false, None);
        };

        if let Some(script_fn) = &binding.script {
            let fail = if let Some(script) = self.script.as_mut() {
                // Script bindings get a null midi sink here; host can call on_init with midi.
                struct NullMidi;
                impl MidiOut for NullMidi {
                    fn send(&mut self, _bytes: &[u8]) {}
                }
                let mut null = NullMidi;
                let mut host = ScriptHost {
                    bus,
                    midi: &mut null,
                    modifiers: &self.modifiers,
                };
                let script_norm = match value {
                    ControlValue::Absolute(n) => n,
                    ControlValue::Relative(d) => d as f32,
                };
                match script.call_named(script_fn, &mut host, script_norm, active) {
                    Ok(()) => None,
                    Err(err) => Some(ScriptBindingFailure {
                        section: section.to_string(),
                        alias: alias.to_string(),
                        script_fn: script_fn.clone(),
                        error: err.to_string(),
                    }),
                }
            } else {
                None
            };
            if is_cc {
                self.cc_last.insert(key.to_string(), Instant::now());
            }
            return (true, fail);
        }

        let action = match &binding.action {
            Some(a) => a.as_str(),
            None => return (false, None),
        };
        let soft = binding.soft_takeover_effective();
        let mut value = value;
        if binding.invert_effective() {
            value = match value {
                ControlValue::Absolute(n) => ControlValue::Absolute(1.0 - n),
                ControlValue::Relative(d) => ControlValue::Relative(-d),
            };
        }
        let Some(routed) = resolve_action(action, section, value, active, soft, &self.snapshot)
        else {
            return (false, None);
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
                self.mirror_engine_cmd(o.clone(), kind, body, midi);
                bus.publish_engine(o.clone(), kind.clone(), body.clone());
            }
            RoutedAction::LibraryEvt { origin, kind, body } => {
                bus.publish_library(origin.clone(), kind.clone(), body.clone());
            }
        }
        (true, None)
    }

    /// Mirror a routed engine cmd into the local snapshot, then refresh the
    /// affected section's LEDs. The engine stays the source of truth; its
    /// `DeckUpdated` / `EngineStatus` mirror corrects any drift.
    fn mirror_engine_cmd(
        &mut self,
        origin: Origin,
        kind: &Kind,
        body: &CmdBody,
        midi: &mut impl MidiOut,
    ) {
        if let Origin::Deck(d) = origin {
            let i = deck_slot(d);
            match body {
                CmdBody::SetPadMode { mode } => self.snapshot.pad_mode[i] = *mode,
                CmdBody::SetTempoRange { tempo_range } => {
                    self.snapshot.tempo_range[i] = *tempo_range;
                }
                CmdBody::SetHeadphoneCue { enabled } => {
                    self.snapshot.headphone_cue[i] = *enabled;
                }
                _ => {}
            }
            match kind {
                Kind::ToggleHeadphoneCue => {
                    self.snapshot.headphone_cue[i] = !self.snapshot.headphone_cue[i]
                }
                Kind::ToggleSync => self.snapshot.sync[i] = !self.snapshot.sync[i],
                Kind::ToggleQuantize => self.snapshot.quantize[i] = !self.snapshot.quantize[i],
                Kind::BeginCueHold => self.snapshot.cue_hold[i] = true,
                Kind::EndCueHold => self.snapshot.cue_hold[i] = false,
                Kind::Play | Kind::TriggerHotCue => self.set_playing_deck(d, true),
                Kind::Pause => self.set_playing_deck(d, false),
                Kind::LoopOut => self.snapshot.loop_active[i] = true,
                Kind::ExitLoop => self.snapshot.loop_active[i] = false,
                // A filled hot cue starts playback; an empty one saves the cue.
                Kind::HotCuePadPress => {
                    if let CmdBody::HotCuePadPress { slot, shift: false } = body {
                        let filled = self.snapshot.hot_cues[i]
                            .get(*slot as usize)
                            .copied()
                            .flatten()
                            .is_some();
                        if filled {
                            self.set_playing_deck(d, true);
                        }
                    }
                }
                _ => {}
            }
            self.refresh_deck_leds(d, midi);
        } else if origin == Origin::Mixer {
            match kind {
                Kind::ToggleMasterCue => self.snapshot.master_cue = !self.snapshot.master_cue,
                _ => {
                    if let CmdBody::SetMasterCue { enabled } = body {
                        self.snapshot.master_cue = *enabled;
                    }
                }
            }
            self.refresh_leds(crate::device::SECTION_MASTER, midi);
        }
    }

    fn refresh_deck_leds(&mut self, deck: u16, midi: &mut impl MidiOut) {
        let section = format!("deck_{}", deck.min(3) + 1);
        self.refresh_leds(&section, midi);
    }

    /// Re-resolve every `signal` binding in `section`; send only what changed.
    fn refresh_leds(&mut self, section: &str, midi: &mut impl MidiOut) {
        let resolved = self.resolve_section(section);
        for (alias, state) in resolved {
            self.apply_led(section, &alias, state, false, midi);
        }
    }

    /// Non-forced sweep of every mapped section (modifier changes).
    fn refresh_all_sections(&mut self, midi: &mut impl MidiOut) {
        let sections: Vec<String> = self.bundle.map.outputs.keys().cloned().collect();
        for section in &sections {
            self.refresh_leds(section, midi);
        }
    }

    /// Forced sweep of every mapped section (attach / reconnect).
    fn refresh_all_leds(&mut self, midi: &mut impl MidiOut) {
        let sections: Vec<String> = self.bundle.map.outputs.keys().cloned().collect();
        for section in &sections {
            for (alias, state) in self.resolve_section(section) {
                self.apply_led(section, &alias, state, true, midi);
            }
        }
    }

    /// Send every mapped lamp's `off` bytes (detach, shutdown).
    fn clear_leds(&mut self, midi: &mut impl MidiOut) {
        let sections: Vec<String> = self.bundle.map.outputs.keys().cloned().collect();
        for section in &sections {
            let aliases: Vec<String> = self
                .bundle
                .map
                .outputs
                .get(section)
                .map(|s| s.keys().cloned().collect())
                .unwrap_or_default();
            for alias in aliases {
                self.apply_led(section, &alias, LedState::Off, true, midi);
            }
        }
        self.output_state.clear();
    }

    /// `(alias, state)` for every output in `section` that declares a `signal`.
    fn resolve_section(&self, section: &str) -> Vec<(String, LedState)> {
        let Some(sec) = self.bundle.map.outputs.get(section) else {
            return Vec::new();
        };
        sec.iter()
            .filter_map(|(alias, out)| {
                let signal = out.signal.as_deref()?;
                let active = self.resolve_signal(section, signal)?;
                let state = match (active, &out.blink) {
                    (false, _) => LedState::Off,
                    (true, Some(_)) => LedState::Blink,
                    (true, None) => LedState::On,
                };
                Some((alias.clone(), state))
            })
            .collect()
    }

    /// `shift_held` lives in the modifier set, not the snapshot.
    fn resolve_signal(&self, section: &str, signal: &str) -> Option<bool> {
        if signal == "shift_held" {
            return Some(self.shift_held(section));
        }
        self.snapshot.signal(section, signal)
    }

    fn shift_held(&self, section: &str) -> bool {
        crate::device::deck_index(section)
            .is_some_and(|n| self.modifiers.contains(&format!("custom.shift_deck{n}")))
    }

    /// Advance the blink phase; re-send blinking LEDs. Call from the MIDI pump.
    pub fn tick_leds(&mut self, midi: &mut impl MidiOut) {
        if !self.has_blink {
            return;
        }
        let now = Instant::now();
        if self
            .last_blink
            .is_some_and(|t| now.duration_since(t) < BLINK_HALF_PERIOD)
        {
            return;
        }
        self.last_blink = Some(now);
        self.blink_on = !self.blink_on;
        let blinking: Vec<(String, String)> = self
            .output_state
            .iter()
            .filter(|(_, state)| **state == LedState::Blink)
            .filter_map(|(key, _)| {
                key.split_once('.')
                    .map(|(s, a)| (s.to_string(), a.to_string()))
            })
            .collect();
        for (section, alias) in blinking {
            if let Some(bytes) = self.blink_bytes(&section, &alias) {
                midi.send(&bytes);
            }
        }
    }

    /// Blink phase bytes: lit target while `blink_on`, else the `off` target.
    fn blink_bytes(&self, section: &str, alias: &str) -> Option<[u8; 3]> {
        let out = self.bundle.map.outputs.get(section)?.get(alias)?;
        let target = if self.blink_on {
            out.blink.as_ref().unwrap_or(&out.on)
        } else {
            &out.off
        };
        resolve_output_bytes(&self.bundle.device, section, target)
    }

    /// Send `state` unless it already matches the cache. Uncached entries count
    /// as [`LedState::Off`] so a fresh session does not spam every lamp.
    fn apply_led(
        &mut self,
        section: &str,
        alias: &str,
        state: LedState,
        force: bool,
        midi: &mut impl MidiOut,
    ) {
        let key = format!("{section}.{alias}");
        let cached = self.output_state.get(&key).copied().unwrap_or_default();
        if !force && cached == state {
            return;
        }
        let bytes = self.led_bytes(section, alias, state);
        self.output_state.insert(key, state);
        if let Some(bytes) = bytes {
            midi.send(&bytes);
        }
    }

    fn led_bytes(&self, section: &str, alias: &str, state: LedState) -> Option<[u8; 3]> {
        let out = self.bundle.map.outputs.get(section)?.get(alias)?;
        let target = match state {
            LedState::On => &out.on,
            LedState::Off => &out.off,
            LedState::Blink => out.blink.as_ref()?,
        };
        resolve_output_bytes(&self.bundle.device, section, target)
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

    fn invert_session() -> MappingSession {
        let b = crate::load_bundle(Path::new("tests/fixtures/invert-tempo")).unwrap();
        MappingSession::from_bundle(b).unwrap()
    }

    #[test]
    fn invert_tempo_cc_flips_set_speed_norm() {
        let mut s = invert_session();
        let mut bus = CaptureBus { cmds: vec![] };

        // CC 0 → inverted → 1.0
        s.handle_midi(&[0xB0, 0x14, 0], &mut bus, &mut NullMidi);
        assert_eq!(bus.cmds.len(), 1);
        match &bus.cmds[0].2 {
            CmdBody::SetSpeed {
                speed,
                soft_takeover: false,
            } => assert!((*speed - 1.0).abs() < 1e-5, "speed={speed}"),
            other => panic!("expected SetSpeed, got {other:?}"),
        }

        // Age coalesce window so the max CC publishes immediately.
        if let Some(t) = s.cc_last.get_mut("deck_1.tempo") {
            *t = Instant::now() - CC_COALESCE - Duration::from_millis(1);
        }

        // CC 127 → inverted → 0.0
        s.handle_midi(&[0xB0, 0x14, 127], &mut bus, &mut NullMidi);
        assert_eq!(bus.cmds.len(), 2);
        match &bus.cmds[1].2 {
            CmdBody::SetSpeed {
                speed,
                soft_takeover: false,
            } => assert!((*speed - 0.0).abs() < 1e-5, "speed={speed}"),
            other => panic!("expected SetSpeed, got {other:?}"),
        }
    }

    #[test]
    fn cc_coalesce_keeps_latest_until_flush() {
        let mut s = session();
        let mut bus = CaptureBus { cmds: vec![] };

        // First CC publishes immediately.
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
            CmdBody::SetVolume {
                volume,
                soft_takeover: true,
            } => {
                assert!((*volume - 1.0).abs() < 1e-5, "volume={volume}");
            }
            other => panic!("expected SetVolume, got {other:?}"),
        }
    }

    #[test]
    fn relative_cc_sums_deltas_until_flush() {
        let mut s = session();
        let mut bus = CaptureBus { cmds: vec![] };

        // BINARY_OFFSET 65=+1 — first publishes immediately.
        s.handle_midi(&[0xB0, 0x22, 65], &mut bus, &mut NullMidi);
        assert_eq!(bus.cmds.len(), 1);
        match &bus.cmds[0].2 {
            CmdBody::JogTurn { delta } => assert_eq!(*delta, 1),
            other => panic!("expected JogTurn, got {other:?}"),
        }

        // Burst: +1 then +3 → pending sum 4.
        s.handle_midi(&[0xB0, 0x22, 65], &mut bus, &mut NullMidi);
        s.handle_midi(&[0xB0, 0x22, 67], &mut bus, &mut NullMidi);
        assert_eq!(bus.cmds.len(), 1, "relative burst must coalesce by sum");

        if let Some(t) = s.cc_last.get_mut("deck_1.jog_turn") {
            *t = Instant::now() - CC_COALESCE - Duration::from_millis(1);
        }
        s.flush_coalesced(&mut bus, &mut NullMidi);
        assert_eq!(bus.cmds.len(), 2);
        match &bus.cmds[1].2 {
            CmdBody::JogTurn { delta } => assert_eq!(*delta, 4),
            other => panic!("expected JogTurn sum, got {other:?}"),
        }
    }
}
