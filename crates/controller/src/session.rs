//! Mapping runtime: MIDI in → cmds; snapshot → MIDI out.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use engine_api::key_shift_page_action;
use engine_api::keyboard_page_action;
use engine_api::BeginCueHold;
use engine_api::EndCueHold;
use engine_api::Event;
use engine_api::ExitLoop;
use engine_api::FxSlotState;
use engine_api::HotCuePadPress;
use engine_api::LoopIn;
use engine_api::LoopOut;
use engine_api::PadMode;
use engine_api::Pause;
use engine_api::PitchPadAction;
use engine_api::Play;
use engine_api::SetFxUnitParam;
use engine_api::SetFxUnitSlot;
use engine_api::SetFxUnitSlotEnabled;
use engine_api::SetFxUnitWet;
use engine_api::SetHeadphoneCue;
use engine_api::SetKeyShift;
use engine_api::SetKeyShiftPage;
use engine_api::SetKeyboardPage;
use engine_api::SetKeyboardRoot;
use engine_api::SetMasterCue;
use engine_api::SetPadMode;
use engine_api::SetTempoRange;
use engine_api::ToggleHeadphoneCue;
use engine_api::ToggleMasterCue;
use engine_api::ToggleQuantize;
use engine_api::ToggleSync;
use engine_api::TriggerHotCue;
use engine_api::KEYBOARD_PAGE_COUNT;
use engine_api::KEY_SHIFT_PAGE_COUNT;
use library_api::{EvtBody as LibraryEvtBody, Kind as LibraryKind, Origin as LibraryOrigin};

use crate::action::EngineScope;
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
    fn publish_engine(&mut self, event: Event);
    fn publish_library(&mut self, origin: LibraryOrigin, kind: LibraryKind, body: LibraryEvtBody);
}

/// Compatibility alias for engine-only hosts/tests.
pub trait BusPublish: ActionPublish {
    fn publish(&mut self, event: Event) {
        self.publish_engine(event);
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
    /// Last absolute value published per `<section>.<alias>|<action>`.
    ///
    /// The DDJ-400 answers the idle heartbeat's position query with a full dump
    /// every second, so unchanged absolute positions arrive continuously. Republishing
    /// them would fight the UI sliders (a hard set would be reverted within a second)
    /// and burn bus traffic, so identical values are dropped.
    cc_last_value: HashMap<String, f32>,
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
            cc_last_value: HashMap::new(),
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

    /// Test-only: backdate a CC's coalesce timestamp so its next tick publishes
    /// immediately, instead of waiting out the ≤60 Hz window with a real sleep.
    ///
    /// Gated behind `test-utils` (plus `cfg(test)` for in-crate use) so it is not
    /// part of the public API of a production build; the `session_input`
    /// integration test is the only external caller.
    #[cfg(any(test, feature = "test-utils"))]
    #[doc(hidden)]
    pub fn age_cc_coalesce_for_test(&mut self, key: &str) {
        // Fail loudly on an unknown key: silently leaving the window un-aged
        // would make the test exercise a real-clock coalesce and pass anyway.
        let t = self
            .cc_last
            .get_mut(key)
            .unwrap_or_else(|| panic!("age_cc_coalesce_for_test: no coalesce window for `{key}`"));
        // `checked_sub` guards the theoretical `Instant` underflow in the first
        // milliseconds of process uptime; if it ever fired, the test would
        // silently exercise an un-aged window, so fail loudly instead.
        *t = Instant::now()
            .checked_sub(CC_COALESCE + Duration::from_millis(1))
            .expect("Instant underflowed ageing the coalesce window");
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
        // Host already ORs pending-loop-in with active; mirror verbatim.
        self.snapshot.loop_in_set[i] = fb.loop_in_set;
        self.snapshot.track_loaded[i] = fb.track_loaded;
        self.snapshot.loop_slots[i] = fb.loop_slots;
        self.snapshot.pad_mode[i] = fb.pad_mode;
        self.set_deck_hot_cues(deck, fb.hot_cues, midi);
        if matches!(fb.pad_mode, PadMode::Keyboard | PadMode::KeyShift) {
            self.refresh_key_shift_leds(deck, midi);
        }
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

    /// Mirror engine `pad_mode` so MIDI `pad n` matches the UI.
    pub fn set_deck_pad_mode(&mut self, deck: u16, mode: PadMode, midi: &mut impl MidiOut) {
        let i = deck_slot(deck);
        let mode_changed = self.snapshot.pad_mode[i] != mode;
        self.snapshot.pad_mode[i] = mode;
        if !mode_changed {
            return;
        }
        // Declarative `pad_mode_*` (incl. keyboard/key_shift) + other deck lamps.
        self.refresh_deck_leds(deck, midi);
        if matches!(mode, PadMode::Keyboard | PadMode::KeyShift) {
            self.refresh_key_shift_leds(deck, midi);
        }
    }

    /// Mirror engine key shift; light the matching Keyboard / Key Shift pad LED.
    pub fn set_deck_key_shift(&mut self, deck: u16, semitones: f32, midi: &mut impl MidiOut) {
        let i = deck_slot(deck);
        if self.snapshot.key_shift[i] == semitones {
            return;
        }
        self.snapshot.key_shift[i] = semitones;
        self.refresh_key_shift_leds(deck, midi);
    }

    /// Mirror engine Keyboard pad semitone page (`1..=4`; pad-bank LEDs).
    pub fn set_deck_keyboard_page(&mut self, deck: u16, page: u8, midi: &mut impl MidiOut) {
        let i = deck_slot(deck);
        let page = page.clamp(1, KEYBOARD_PAGE_COUNT);
        if self.snapshot.keyboard_page[i] == page {
            return;
        }
        self.snapshot.keyboard_page[i] = page;
        self.refresh_key_shift_leds(deck, midi);
    }

    /// Mirror engine Key Shift pad semitone page (`1..=5`; pad-bank LEDs).
    pub fn set_deck_key_shift_page(&mut self, deck: u16, page: u8, midi: &mut impl MidiOut) {
        let i = deck_slot(deck);
        let page = page.clamp(1, KEY_SHIFT_PAGE_COUNT);
        if self.snapshot.key_shift_page[i] == page {
            return;
        }
        self.snapshot.key_shift_page[i] = page;
        self.refresh_key_shift_leds(deck, midi);
    }

    /// Mirror engine Keyboard pad root hot cue.
    pub fn set_deck_keyboard_root(&mut self, deck: u16, slot: u8) {
        let i = deck_slot(deck);
        self.snapshot.keyboard_root_hot_cue[i] = slot;
    }

    /// Re-send the Keyboard or Key Shift pad-bank LEDs for the deck's current mode.
    ///
    /// Exactly the pad whose page semitone matches the current key shift is lit;
    /// page-5 action pads (Reset / Up / Down / Sync) have no absolute match, so a
    /// page with no matching absolute semitone leaves every pad dark. Force-resent
    /// (HW often clears pad LEDs on page switch). Signal-less `[outputs.*]` entries
    /// are driven via [`Self::apply_output_signal`].
    pub fn refresh_key_shift_leds(&mut self, deck: u16, midi: &mut impl MidiOut) {
        let i = deck_slot(deck);
        let section = format!("deck_{}", deck + 1);
        let shift = self.snapshot.key_shift[i];
        let mode = self.snapshot.pad_mode[i];
        let alias_prefix = match mode {
            PadMode::KeyShift => "key_shift_pad",
            PadMode::Keyboard => "keyboard_pad",
            _ => return,
        };
        let actions: [PitchPadAction; 8] = std::array::from_fn(|slot| match mode {
            PadMode::KeyShift => key_shift_page_action(self.snapshot.key_shift_page[i], slot as u8),
            _ => keyboard_page_action(self.snapshot.keyboard_page[i], slot as u8),
        });
        for (slot, action) in actions.iter().enumerate() {
            let alias = format!("{alias_prefix}_{}", slot + 1);
            let active = matches!(action, PitchPadAction::Semitone(s) if (f32::from(*s) - shift).abs() < 0.5);
            self.output_state.remove(&format!("{section}.{alias}"));
            self.apply_output_signal(&section, &alias, active, midi);
        }
    }

    /// Mirror mixer state (MASTER CUE LED).
    pub fn set_master_cue(&mut self, enabled: bool, midi: &mut impl MidiOut) {
        self.snapshot.master_cue = enabled;
        self.refresh_leds(crate::device::SECTION_MASTER, midi);
    }

    /// Mirror the shared Beat FX unit slot (host `DeckUpdated` / attach replay).
    pub fn set_fx_state(&mut self, fx: &FxSlotState, midi: &mut impl MidiOut) {
        if self.snapshot.fx == *fx {
            return;
        }
        self.snapshot.fx = fx.clone();
        self.refresh_leds(crate::device::SECTION_MASTER, midi);
    }

    /// State-only mirror; use the `set_deck_*` setters to also refresh LEDs.
    pub fn set_control_value(&mut self, scope: EngineScope, key: &str, value: f32) {
        if key == "playing" {
            if let EngineScope::Deck(d) = scope {
                self.set_playing_deck(d, value > 0.5);
                return;
            }
        }
        self.snapshot.set_value(scope, key, value);
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
    // shortcut: internal MIDI dispatch; packing into a struct is noise for CI threshold (7).
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
            if let ControlValue::Absolute(norm) = value {
                let dedupe_key = format!("{key}|{action}");
                if self.cc_last_value.get(&dedupe_key) == Some(&norm) {
                    self.cc_last.insert(key.to_string(), Instant::now());
                    return (true, None);
                }
                self.cc_last_value.insert(dedupe_key, norm);
            }
            self.cc_last.insert(key.to_string(), Instant::now());
        }
        match &routed {
            RoutedAction::EngineCmd(event) => {
                self.publish_engine_cmds(std::slice::from_ref(event), bus, midi);
            }
            // Exclusive groups (e.g. CH SELECT) publish every cmd in order.
            RoutedAction::EngineCmds(events) => self.publish_engine_cmds(events, bus, midi),
            RoutedAction::LibraryEvt { origin, kind, body } => {
                bus.publish_library(origin.clone(), kind.clone(), body.clone());
            }
        }
        (true, None)
    }

    /// Mirror a routed engine command group into the snapshot, then publish it.
    ///
    /// Single-command actions and exclusive groups share this, so the
    /// mirror-then-publish order (and the clones it needs) is defined once.
    fn publish_engine_cmds(
        &mut self,
        events: &[Event],
        bus: &mut impl ActionPublish,
        midi: &mut impl MidiOut,
    ) {
        for event in events {
            self.mirror_engine_cmd(event, midi);
            bus.publish_engine(event.clone());
        }
    }

    /// Mirror a routed engine cmd into the local snapshot, then refresh the
    /// affected section's LEDs. The engine stays the source of truth; its
    /// `DeckUpdated` / `EngineSnapshot` mirror corrects any drift.
    fn mirror_engine_cmd(&mut self, event: &Event, midi: &mut impl MidiOut) {
        match event {
            Event::SetPadMode(SetPadMode { deck, mode }) => {
                self.snapshot.pad_mode[deck_slot(*deck)] = *mode;
                self.refresh_deck_cmd_leds(*deck, midi);
            }
            Event::SetKeyShift(SetKeyShift { deck, semitones }) => {
                self.snapshot.key_shift[deck_slot(*deck)] = *semitones;
                self.refresh_deck_cmd_leds(*deck, midi);
            }
            Event::SetKeyboardPage(SetKeyboardPage { deck, page }) => {
                self.snapshot.keyboard_page[deck_slot(*deck)] =
                    (*page).clamp(1, KEYBOARD_PAGE_COUNT);
                self.refresh_deck_cmd_leds(*deck, midi);
            }
            Event::SetKeyShiftPage(SetKeyShiftPage { deck, page }) => {
                self.snapshot.key_shift_page[deck_slot(*deck)] =
                    (*page).clamp(1, KEY_SHIFT_PAGE_COUNT);
                self.refresh_deck_cmd_leds(*deck, midi);
            }
            Event::SetKeyboardRoot(SetKeyboardRoot { deck, slot }) => {
                self.snapshot.keyboard_root_hot_cue[deck_slot(*deck)] = *slot;
                self.refresh_deck_cmd_leds(*deck, midi);
            }
            Event::SetHeadphoneCue(SetHeadphoneCue { deck, enabled }) => {
                self.snapshot.headphone_cue[deck_slot(*deck)] = *enabled;
                self.refresh_deck_cmd_leds(*deck, midi);
            }
            Event::ToggleHeadphoneCue(ToggleHeadphoneCue { deck }) => {
                let i = deck_slot(*deck);
                self.snapshot.headphone_cue[i] = !self.snapshot.headphone_cue[i];
                self.refresh_deck_cmd_leds(*deck, midi);
            }
            Event::ToggleSync(ToggleSync { deck, .. }) => {
                let i = deck_slot(*deck);
                self.snapshot.sync[i] = !self.snapshot.sync[i];
                self.refresh_deck_cmd_leds(*deck, midi);
            }
            Event::ToggleQuantize(ToggleQuantize { deck }) => {
                let i = deck_slot(*deck);
                self.snapshot.quantize[i] = !self.snapshot.quantize[i];
                self.refresh_deck_cmd_leds(*deck, midi);
            }
            Event::BeginCueHold(BeginCueHold { deck }) => {
                self.snapshot.cue_hold[deck_slot(*deck)] = true;
                self.refresh_deck_cmd_leds(*deck, midi);
            }
            Event::EndCueHold(EndCueHold { deck }) => {
                self.snapshot.cue_hold[deck_slot(*deck)] = false;
                self.refresh_deck_cmd_leds(*deck, midi);
            }
            Event::Play(Play { deck }) | Event::TriggerHotCue(TriggerHotCue { deck, .. }) => {
                self.set_playing_deck(*deck, true);
                self.refresh_deck_cmd_leds(*deck, midi);
            }
            Event::Pause(Pause { deck }) => {
                self.set_playing_deck(*deck, false);
                self.refresh_deck_cmd_leds(*deck, midi);
            }
            Event::LoopIn(LoopIn { deck, .. }) => {
                self.snapshot.loop_in_set[deck_slot(*deck)] = true;
                self.refresh_deck_cmd_leds(*deck, midi);
            }
            Event::LoopOut(LoopOut { deck, .. }) => {
                let i = deck_slot(*deck);
                self.snapshot.loop_active[i] = true;
                self.snapshot.loop_in_set[i] = true;
                self.refresh_deck_cmd_leds(*deck, midi);
            }
            Event::ExitLoop(ExitLoop { deck }) => {
                let i = deck_slot(*deck);
                self.snapshot.loop_active[i] = false;
                self.snapshot.loop_in_set[i] = false;
                self.refresh_deck_cmd_leds(*deck, midi);
            }
            // A filled hot cue starts playback; an empty one saves the cue.
            Event::HotCuePadPress(HotCuePadPress {
                deck,
                slot,
                shift: false,
            }) => {
                let filled = self.snapshot.hot_cues[deck_slot(*deck)]
                    .get(*slot as usize)
                    .copied()
                    .flatten()
                    .is_some();
                if filled {
                    self.set_playing_deck(*deck, true);
                    self.refresh_deck_cmd_leds(*deck, midi);
                }
            }
            // State-only: no LED signal reads the tempo range, so no refresh.
            Event::SetTempoRange(SetTempoRange { deck, tempo_range }) => {
                self.snapshot.tempo_range[deck_slot(*deck)] = *tempo_range;
            }
            // Mixer strip: the master section always re-resolves (an FX or cue
            // lamp may have moved).
            Event::SetCrossfader(_)
            | Event::SetMasterVolume(_)
            | Event::SetCueVolume(_)
            | Event::SetCueMix(_) => self.refresh_leds(crate::device::SECTION_MASTER, midi),
            Event::ToggleMasterCue(ToggleMasterCue {}) => {
                self.snapshot.master_cue = !self.snapshot.master_cue;
                self.refresh_leds(crate::device::SECTION_MASTER, midi);
            }
            Event::SetMasterCue(SetMasterCue { enabled }) => {
                self.snapshot.master_cue = *enabled;
                self.refresh_leds(crate::device::SECTION_MASTER, midi);
            }
            Event::SetFxUnitSlotEnabled(SetFxUnitSlotEnabled { enabled, .. }) => {
                self.snapshot.fx.enabled = *enabled;
                self.refresh_leds(crate::device::SECTION_MASTER, midi);
            }
            // Selecting an effect reloads its knobs, but `FxSlot::set_kind`
            // keeps the slot's armed flag, so the mirror must keep it too
            // (clearing it would darken the lamps until the engine echo).
            Event::SetFxUnitSlot(SetFxUnitSlot { kind: next, .. }) => {
                if self.snapshot.fx.kind != *next {
                    self.snapshot.fx.kind = *next;
                    self.snapshot.fx.params = FxSlotState::boot().params;
                }
                self.refresh_leds(crate::device::SECTION_MASTER, midi);
            }
            // Tracked so rapid BEAT presses step from the pending division.
            Event::SetFxUnitParam(SetFxUnitParam { param, value, .. }) => {
                if let Some(slot) = self.snapshot.fx.params.get_mut(*param as usize) {
                    *slot = *value;
                }
                self.refresh_leds(crate::device::SECTION_MASTER, midi);
            }
            Event::SetFxUnitWet(SetFxUnitWet { wet, .. }) => {
                self.snapshot.fx.wet = *wet;
                self.refresh_leds(crate::device::SECTION_MASTER, midi);
            }
            _ => {}
        }
    }

    /// Deck lamps after a mirrored command, plus the key-shift bank lamps when
    /// that bank owns the pads.
    fn refresh_deck_cmd_leds(&mut self, deck: u16, midi: &mut impl MidiOut) {
        self.refresh_deck_leds(deck, midi);
        if matches!(
            self.snapshot.pad_mode[deck_slot(deck)],
            PadMode::Keyboard | PadMode::KeyShift
        ) {
            self.refresh_key_shift_leds(deck, midi);
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
        // Drop the cache first so a panic / early exit mid-send cannot leave
        // stale On/Blink entries that would skip the next refresh.
        self.output_state.clear();
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
            // Only called for `LedState::Blink`, which requires `blink: Some`.
            out.blink.as_ref()?
        } else {
            &out.off
        };
        resolve_output_bytes(&self.bundle.device, section, target)
    }

    /// Host/engine playing mirror (play LED via declarative `playing` signal).
    pub fn on_deck_playing(&mut self, deck: u16, playing: bool, midi: &mut impl MidiOut) {
        self.set_playing_deck(deck, playing);
        self.refresh_deck_leds(deck, midi);
    }

    /// Drive a signal-less `[outputs.*]` entry (Keyboard / Key Shift pad banks).
    /// Maps `active` onto [`LedState::On`] / [`LedState::Off`] through the same
    /// send helper as declarative refresh. When the cache already holds `state`,
    /// this is a no-op; otherwise it force-applies via [`Self::apply_led`] so a
    /// prior `output_state.remove` still re-sends Off.
    pub fn apply_output_signal(
        &mut self,
        section: &str,
        alias: &str,
        active: bool,
        midi: &mut impl MidiOut,
    ) {
        let state = if active { LedState::On } else { LedState::Off };
        let key = format!("{section}.{alias}");
        if self.output_state.get(&key).copied() == Some(state) {
            return;
        }
        self.apply_led(section, alias, state, true, midi);
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
        cmds: Vec<Event>,
    }

    impl ActionPublish for CaptureBus {
        fn publish_engine(&mut self, event: Event) {
            self.cmds.push(event);
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
        match &bus.cmds[0] {
            Event::SetSpeed(engine_api::SetSpeed {
                deck: 0,
                speed,
                soft_takeover: false,
            }) => assert!((*speed - 1.0).abs() < 1e-5, "speed={speed}"),
            other => panic!("expected SetSpeed, got {other:?}"),
        }

        // Age coalesce window so the max CC publishes immediately.
        s.age_cc_coalesce_for_test("deck_1.tempo");

        // CC 127 → inverted → 0.0
        s.handle_midi(&[0xB0, 0x14, 127], &mut bus, &mut NullMidi);
        assert_eq!(bus.cmds.len(), 2);
        match &bus.cmds[1] {
            Event::SetSpeed(engine_api::SetSpeed {
                deck: 0,
                speed,
                soft_takeover: false,
            }) => assert!((*speed - 0.0).abs() < 1e-5, "speed={speed}"),
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
        s.age_cc_coalesce_for_test("deck_1.volume");
        s.flush_coalesced(&mut bus, &mut NullMidi);
        assert_eq!(bus.cmds.len(), 2);
        match &bus.cmds[1] {
            Event::SetVolume(engine_api::SetVolume {
                deck: 0,
                volume,
                soft_takeover: true,
            }) => {
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
        match &bus.cmds[0] {
            Event::JogTurn(engine_api::JogTurn { deck: 0, delta }) => assert_eq!(*delta, 1),
            other => panic!("expected JogTurn, got {other:?}"),
        }

        // Burst: +1 then +3 → pending sum 4.
        s.handle_midi(&[0xB0, 0x22, 65], &mut bus, &mut NullMidi);
        s.handle_midi(&[0xB0, 0x22, 67], &mut bus, &mut NullMidi);
        assert_eq!(bus.cmds.len(), 1, "relative burst must coalesce by sum");

        s.age_cc_coalesce_for_test("deck_1.jog_turn");
        s.flush_coalesced(&mut bus, &mut NullMidi);
        assert_eq!(bus.cmds.len(), 2);
        match &bus.cmds[1] {
            Event::JogTurn(engine_api::JogTurn { deck: 0, delta }) => assert_eq!(*delta, 4),
            other => panic!("expected JogTurn sum, got {other:?}"),
        }
    }

    #[test]
    fn coalesced_absolute_turn_drops_a_return_to_the_published_value() {
        let mut s = session();
        let mut bus = CaptureBus { cmds: vec![] };

        // A publishes immediately and becomes the dedupe baseline.
        s.handle_midi(&[0xB0, 0x13, 64], &mut bus, &mut NullMidi);
        assert_eq!(bus.cmds.len(), 1);

        // B then A arrive inside the coalesce window; only the latest (A) stays
        // pending, so the flush returns to the value already in the engine.
        s.handle_midi(&[0xB0, 0x13, 96], &mut bus, &mut NullMidi);
        s.handle_midi(&[0xB0, 0x13, 64], &mut bus, &mut NullMidi);
        s.age_cc_coalesce_for_test("deck_1.volume");
        s.flush_coalesced(&mut bus, &mut NullMidi);
        assert_eq!(
            bus.cmds.len(),
            1,
            "returning to the published value publishes nothing"
        );

        // The baseline is the flushed value, so the next real move still lands.
        s.age_cc_coalesce_for_test("deck_1.volume");
        s.handle_midi(&[0xB0, 0x13, 96], &mut bus, &mut NullMidi);
        assert_eq!(bus.cmds.len(), 2);
    }

    #[test]
    fn repeated_absolute_cc_value_is_not_republished() {
        let mut s = session();
        let mut bus = CaptureBus { cmds: vec![] };

        s.handle_midi(&[0xB0, 0x13, 64], &mut bus, &mut NullMidi);
        assert_eq!(bus.cmds.len(), 1);

        // The DDJ-400 answers the idle heartbeat with a full position dump every
        // second; re-publishing an unchanged value would fight the GUI sliders.
        s.age_cc_coalesce_for_test("deck_1.volume");
        s.handle_midi(&[0xB0, 0x13, 64], &mut bus, &mut NullMidi);
        s.flush_coalesced(&mut bus, &mut NullMidi);
        assert_eq!(
            bus.cmds.len(),
            1,
            "unchanged absolute value must be dropped"
        );

        // A real move still reaches the engine.
        s.age_cc_coalesce_for_test("deck_1.volume");
        s.handle_midi(&[0xB0, 0x13, 65], &mut bus, &mut NullMidi);
        assert_eq!(bus.cmds.len(), 2);
    }
}
