//! Mapping runtime: MIDI in → cmds; snapshot → MIDI out.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use engine_api::{CmdBody, Kind, Origin};
use library_api::{EvtBody as LibraryEvtBody, Kind as LibraryKind, Origin as LibraryOrigin};

use crate::action::{resolve_action, ControlSnapshot, RoutedAction};
use crate::action_id::{bind_origin, parse_action_id, BoundOrigin};
use crate::bundle::Bundle;
use crate::device::SECTION_CUSTOM;
use crate::error::{LoadError, MidiPortError, RuntimeError};
use crate::map_file::{InputBinding, OutputTarget};
use crate::midi::{norm_from_cc14, parse_short, MidiEndpoint, ShortMsg};
use crate::script::{ScriptHost, ScriptRuntime};

pub const SOFT_TAKEOVER_THRESHOLD: f32 = 3.0 / 127.0;
const CC_COALESCE: Duration = Duration::from_nanos(1_000_000_000 / 60);

pub trait ActionPublish {
    fn publish_engine(&mut self, origin: Origin, kind: Kind, body: CmdBody);
    fn publish_library_evt(
        &mut self,
        origin: LibraryOrigin,
        kind: LibraryKind,
        body: LibraryEvtBody,
    );
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
    pub bundle: Bundle,
    snapshot: ControlSnapshot,
    /// Active custom.* modifiers (held).
    modifiers: HashSet<String>,
    /// Soft-takeover latched keys: "section.alias".
    soft_latched: HashSet<String>,
    /// Last CC publish time per "section.alias".
    cc_last: HashMap<String, Instant>,
    /// 14-bit CC pair state: "section.alias" → (msb, lsb).
    cc14_state: HashMap<String, (Option<u8>, Option<u8>)>,
    /// Last edge state for notes: "section.alias" → active.
    note_state: HashMap<String, bool>,
    /// Output signal cache: "section.alias" → active.
    output_state: HashMap<String, bool>,
    script: Option<ScriptRuntime>,
}

impl MappingSession {
    pub fn from_bundle(bundle: Bundle) -> Result<Self, LoadError> {
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
            cc14_state: HashMap::new(),
            note_state: HashMap::new(),
            output_state: HashMap::new(),
            script,
        })
    }

    pub fn snapshot(&self) -> &ControlSnapshot {
        &self.snapshot
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
        self.run_hook("on_init", bus, midi)
    }

    pub fn on_shutdown(
        &mut self,
        bus: &mut impl ActionPublish,
        midi: &mut impl MidiOut,
    ) -> Result<(), RuntimeError> {
        self.run_hook("on_shutdown", bus, midi)
    }

    fn run_hook(
        &mut self,
        name: &str,
        bus: &mut impl ActionPublish,
        midi: &mut impl MidiOut,
    ) -> Result<(), RuntimeError> {
        let Some(script) = self.script.as_mut() else {
            return Ok(());
        };
        let mut host = ScriptHost {
            bus,
            midi,
            snapshot: &self.snapshot,
            modifiers: &self.modifiers,
        };
        script.call_hook(name, &mut host)
    }

    pub fn handle_midi(&mut self, bytes: &[u8], bus: &mut impl ActionPublish) {
        let Some(parsed) = parse_short(bytes) else {
            return;
        };
        let Some((section, alias, ep)) = self.bundle.device.find_input_match(parsed.match_key)
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
            if parsed.active {
                self.modifiers.insert(mod_key);
            } else {
                self.modifiers.remove(&mod_key);
            }
            // custom is not declarative-input bindable; still allow script-only later
            return;
        }

        // Resolve normalized value (cc14 pairs MSB+LSB).
        let mut norm = parsed.norm;
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
            norm = norm_from_cc14(msb, lsb);
        }

        // Edge for notes: only process transitions for button-like msgs.
        if !is_cc {
            let prev = self.note_state.get(&key).copied().unwrap_or(false);
            if prev == parsed.active {
                return;
            }
            self.note_state.insert(key.clone(), parsed.active);
        } else {
            // Coalesce CC publishes ~60Hz (soft-blocked events do not consume budget).
            let now = Instant::now();
            if let Some(last) = self.cc_last.get(&key) {
                if now.duration_since(*last) < CC_COALESCE {
                    return;
                }
            }
        }

        let bindings = self.bundle.map.bindings_for(&section, &alias);
        if bindings.is_empty() {
            return;
        }
        let binding = select_binding(&bindings, &self.modifiers);
        let Some(binding) = binding else {
            return;
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
                let _ = script.call_named(script_fn, &mut host, norm, parsed.active);
            }
            return;
        }

        let action = match &binding.action {
            Some(a) => a.as_str(),
            None => return,
        };
        let Ok((template, leaf)) = parse_action_id(action) else {
            return;
        };
        let Ok(bound) = bind_origin(template, &section) else {
            return;
        };

        if binding.soft_takeover_effective() {
            if let BoundOrigin::Engine(origin) = &bound {
                if let Some(engine_v) = self.snapshot.get_norm_for_action(origin.clone(), leaf) {
                    let latched = self.soft_latched.contains(&key);
                    if !latched {
                        let dist = (norm - engine_v).abs();
                        if dist > SOFT_TAKEOVER_THRESHOLD {
                            return;
                        }
                        self.soft_latched.insert(key.clone());
                    }
                }
            }
        }

        if let Some(routed) = resolve_action(action, &section, norm, parsed.active, &self.snapshot)
        {
            if is_cc {
                self.cc_last.insert(key.clone(), Instant::now());
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
                                self.snapshot.volume[d as usize] = *volume;
                            }
                        }
                        CmdBody::SetFilter { filter_db } => {
                            if let Origin::Deck(d) = *o {
                                self.snapshot.filter_db[d as usize] = *filter_db;
                            }
                        }
                        CmdBody::SetGainTrim { gain_db } => {
                            if let Origin::Deck(d) = *o {
                                self.snapshot.gain_db[d as usize] = *gain_db;
                            }
                        }
                        CmdBody::SetSpeed { speed } => {
                            if let Origin::Deck(d) = *o {
                                self.snapshot.speed[d as usize] = *speed;
                            }
                        }
                        CmdBody::SetEq { low, mid, high } => {
                            if let Origin::Deck(d) = *o {
                                let i = d as usize;
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
                        _ => {}
                    }
                    if matches!(kind, Kind::Play) {
                        if let Origin::Deck(d) = *o {
                            self.snapshot.playing[d as usize] = true;
                        }
                    }
                    if matches!(kind, Kind::Pause) {
                        if let Origin::Deck(d) = *o {
                            self.snapshot.playing[d as usize] = false;
                        }
                    }
                    bus.publish_engine(o.clone(), kind.clone(), body.clone());
                }
                RoutedAction::LibraryEvt { origin, kind, body } => {
                    bus.publish_library_evt(origin.clone(), kind.clone(), body.clone());
                }
            }
        }
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

    #[test]
    fn soft_threshold_is_3_of_127() {
        assert!((SOFT_TAKEOVER_THRESHOLD - 3.0 / 127.0).abs() < 1e-9);
    }
}
