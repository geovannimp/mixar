//! Soft-takeover gate for absolute MIDI controls (engine-owned latch).

use std::collections::HashMap;

/// Match controller’s former threshold (3/127 of full scale).
pub const SOFT_TAKEOVER_THRESHOLD: f32 = 3.0 / 127.0;

const STRIP_DB_MIN: f32 = -24.0;
const STRIP_DB_MAX: f32 = 24.0;

pub fn db_to_norm(db: f32) -> f32 {
    ((db - STRIP_DB_MIN) / (STRIP_DB_MAX - STRIP_DB_MIN)).clamp(0.0, 1.0)
}

/// Pioneer tempo fader mapping (matches controller).
pub fn speed_to_norm(speed: f32) -> f32 {
    let s = speed.clamp(0.84, 1.16);
    (1.0 - (s - 0.84) / 0.32).clamp(0.0, 1.0)
}

#[derive(Clone, Debug, Default)]
pub struct SoftTakeoverState {
    /// Latched when HW has caught engine value (hard-set clears latch).
    latched: HashMap<String, bool>,
}

impl SoftTakeoverState {
    pub fn allow(
        &mut self,
        key: &str,
        soft_takeover: bool,
        current_norm: f32,
        incoming_norm: f32,
    ) -> bool {
        if !soft_takeover {
            self.latched.insert(key.to_string(), false);
            return true;
        }
        let latched = self.latched.get(key).copied().unwrap_or(false);
        if latched {
            return true;
        }
        if (incoming_norm - current_norm).abs() <= SOFT_TAKEOVER_THRESHOLD {
            self.latched.insert(key.to_string(), true);
            return true;
        }
        false
    }
}

pub fn key_deck(deck_id: usize, control: &str) -> String {
    format!("deck{deck_id}.{control}")
}

pub fn key_mixer(control: &str) -> String {
    format!("mixer.{control}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_until_near_then_latches() {
        let mut s = SoftTakeoverState::default();
        let key = "deck0.volume";
        assert!(!s.allow(key, true, 0.9, 0.0));
        assert!(!s.allow(key, true, 0.9, 0.5));
        assert!(s.allow(key, true, 0.9, 0.9));
        assert!(s.allow(key, true, 0.9, 0.0), "latched → pass");
    }

    #[test]
    fn hard_set_clears_latch() {
        let mut s = SoftTakeoverState::default();
        let key = "deck0.volume";
        assert!(s.allow(key, true, 0.5, 0.5));
        assert!(s.allow(key, false, 0.5, 1.0));
        assert!(!s.allow(key, true, 0.5, 1.0));
    }
}
