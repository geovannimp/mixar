use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OnNonZeroAction {
    Nothing,
    #[default]
    Play,
    TogglePlay,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OnZeroAction {
    Nothing,
    Pause,
    #[default]
    PauseAndCue,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct FaderStartConfig {
    pub enabled: bool,
    pub threshold_pct: u8,
    pub on_non_zero: OnNonZeroAction,
    pub on_zero: OnZeroAction,
}

impl Default for FaderStartConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            threshold_pct: 5,
            on_non_zero: OnNonZeroAction::Play,
            on_zero: OnZeroAction::PauseAndCue,
        }
    }
}

impl FaderStartConfig {
    pub fn clamp(self) -> Self {
        Self {
            threshold_pct: self.threshold_pct.clamp(1, 20),
            ..self
        }
    }

    pub fn zero_band(&self) -> f32 {
        self.threshold_pct as f32 / 100.0
    }

    pub fn release_band(&self) -> f32 {
        (self.threshold_pct + 2).min(100) as f32 / 100.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FaderStartAction {
    Play,
    TogglePlay,
    Pause,
    PauseAndCue,
}

#[derive(Debug, Default)]
pub struct FaderStartState {
    at_zero: Vec<Option<bool>>,
}

impl FaderStartState {
    pub fn reset(&mut self) {
        self.at_zero.clear();
    }

    pub fn resolve(
        &mut self,
        cfg: &FaderStartConfig,
        deck: u16,
        volume: f32,
    ) -> Option<FaderStartAction> {
        let cfg = cfg.clamp();
        let idx = deck as usize;
        if self.at_zero.len() <= idx {
            self.at_zero.resize(idx + 1, None);
        }

        let latch = self.at_zero[idx];
        match latch {
            None => {
                self.at_zero[idx] = Some(volume <= cfg.zero_band());
                None
            }
            Some(true) => {
                if volume <= cfg.release_band() {
                    None
                } else {
                    self.at_zero[idx] = Some(false);
                    if cfg.enabled {
                        match cfg.on_non_zero {
                            OnNonZeroAction::Nothing => None,
                            OnNonZeroAction::Play => Some(FaderStartAction::Play),
                            OnNonZeroAction::TogglePlay => Some(FaderStartAction::TogglePlay),
                        }
                    } else {
                        None
                    }
                }
            }
            Some(false) => {
                if volume > cfg.zero_band() {
                    None
                } else {
                    self.at_zero[idx] = Some(true);
                    if cfg.enabled {
                        match cfg.on_zero {
                            OnZeroAction::Nothing => None,
                            OnZeroAction::Pause => Some(FaderStartAction::Pause),
                            OnZeroAction::PauseAndCue => Some(FaderStartAction::PauseAndCue),
                        }
                    } else {
                        None
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> FaderStartConfig {
        FaderStartConfig::default()
    }

    fn enabled() -> FaderStartConfig {
        FaderStartConfig {
            enabled: true,
            ..Default::default()
        }
    }

    #[test]
    fn first_value_only_seeds_the_latch() {
        let mut state = FaderStartState::default();
        assert_eq!(state.resolve(&enabled(), 0, 0.0), None);
    }

    #[test]
    fn leaving_the_band_plays_and_returning_pauses_with_cue() {
        let mut state = FaderStartState::default();
        state.resolve(&enabled(), 0, 0.0);
        assert_eq!(
            state.resolve(&enabled(), 0, 0.42),
            Some(FaderStartAction::Play)
        );
        assert_eq!(
            state.resolve(&enabled(), 0, 0.01),
            Some(FaderStartAction::PauseAndCue)
        );
    }

    #[test]
    fn one_transition_per_crossing() {
        let mut state = FaderStartState::default();
        state.resolve(&enabled(), 0, 0.0);
        assert_eq!(
            state.resolve(&enabled(), 0, 0.9),
            Some(FaderStartAction::Play)
        );
        assert_eq!(state.resolve(&enabled(), 0, 0.91), None);
        assert_eq!(state.resolve(&enabled(), 0, 0.5), None);
    }

    #[test]
    fn release_band_ignores_jitter_above_the_threshold() {
        let mut state = FaderStartState::default();
        state.resolve(&enabled(), 0, 0.0);
        // 6% is inside the 5%..=7% gap: still zero, no play.
        assert_eq!(state.resolve(&enabled(), 0, 0.06), None);
        assert_eq!(
            state.resolve(&enabled(), 0, 0.071),
            Some(FaderStartAction::Play)
        );
    }

    #[test]
    fn disabled_reads_no_actions_but_tracks_the_latch() {
        let mut state = FaderStartState::default();
        let off = cfg();
        state.resolve(&off, 0, 0.0);
        assert_eq!(state.resolve(&off, 0, 0.8), None);
        // Enabling mid-session must not fire from the value the latch already holds.
        assert_eq!(state.resolve(&enabled(), 0, 0.8), None);
        assert_eq!(
            state.resolve(&enabled(), 0, 0.0),
            Some(FaderStartAction::PauseAndCue)
        );
    }

    #[test]
    fn nothing_actions_return_none() {
        let mut state = FaderStartState::default();
        let quiet = FaderStartConfig {
            enabled: true,
            on_non_zero: OnNonZeroAction::Nothing,
            on_zero: OnZeroAction::Nothing,
            ..Default::default()
        };
        state.resolve(&quiet, 0, 0.0);
        assert_eq!(state.resolve(&quiet, 0, 0.9), None);
        assert_eq!(state.resolve(&quiet, 0, 0.0), None);
    }

    #[test]
    fn threshold_is_clamped_and_decks_are_independent() {
        assert_eq!(
            FaderStartConfig {
                threshold_pct: 0,
                ..Default::default()
            }
            .clamp()
            .threshold_pct,
            1
        );
        assert_eq!(
            FaderStartConfig {
                threshold_pct: 99,
                ..Default::default()
            }
            .clamp()
            .threshold_pct,
            20
        );
        let mut state = FaderStartState::default();
        state.resolve(&enabled(), 0, 0.0);
        state.resolve(&enabled(), 3, 0.0);
        assert_eq!(
            state.resolve(&enabled(), 0, 0.9),
            Some(FaderStartAction::Play)
        );
        assert_eq!(
            state.resolve(&enabled(), 3, 0.9),
            Some(FaderStartAction::Play)
        );
    }

    #[test]
    fn reset_clears_every_latch() {
        let mut state = FaderStartState::default();
        state.resolve(&enabled(), 0, 0.0);
        state.reset();
        assert_eq!(state.resolve(&enabled(), 0, 0.9), None);
    }
}
