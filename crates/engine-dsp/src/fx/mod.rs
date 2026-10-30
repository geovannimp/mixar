//! Per-deck pre-fader FX insert chain (`docs/deck-spec.md` §5.9 F2–F4).
//!
//! The chain is the substrate for FX presets: the DDJ-400-style Beat FX bar
//! drives one unit today, and a preset maps to the whole chain
//! ([`FX_SLOT_COUNT`] bounds its length), so the remaining capacity stays
//! available for the preset editor.
//!
//! Adding an effect: new sibling module implementing [`Fx`], one [`FxKind`]
//! variant, one arm in [`FxKind::create`], and one entry in the Dart label
//! tables (`apps/gui-flutter/lib/core/utils/fx_params.dart`).

use audio_core::Sample;

pub mod echo;
pub mod reverb;

pub use echo::Echo;
pub use reverb::Reverb;

/// Chain capacity per deck (bounds a preset's effect count); keep in sync with
/// `bus_api::FX_SLOT_COUNT`.
pub const FX_SLOT_COUNT: usize = 3;
/// Maximum knobs per effect (F3); keep in sync with `bus_api::FX_PARAM_COUNT`.
pub const FX_PARAM_COUNT: usize = 3;

/// Fallback sync tempo when the master deck has no BPM (unanalyzed track).
pub const DEFAULT_FX_BPM: f32 = 120.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FxKind {
    #[default]
    Off,
    Echo,
    Reverb,
}

impl FxKind {
    /// DSP state for this effect; `None` for [`FxKind::Off`].
    pub fn create(self, sample_rate: u32) -> Option<Box<dyn Fx>> {
        match self {
            FxKind::Off => None,
            FxKind::Echo => Some(Box::new(Echo::new(sample_rate))),
            FxKind::Reverb => Some(Box::new(Reverb::new(sample_rate))),
        }
    }
}

/// One effect's DSP state.
pub trait Fx: std::fmt::Debug + Send {
    /// Knob count (`0..=FX_PARAM_COUNT`).
    fn param_count(&self) -> usize;
    /// Default normalized knob positions for a freshly selected effect.
    fn default_params(&self) -> [f32; FX_PARAM_COUNT];
    /// Apply normalized knob `index` (`index < param_count()`); clamps internally.
    fn set_param(&mut self, index: usize, value: f32);
    /// Sync tempo for beat-synced effects; ignored by free-run effects.
    fn set_tempo(&mut self, _bpm: f32) {}
    /// Clear internal state (tails) on slot disable, kind change, or engine reset.
    fn reset(&mut self);
    /// Wet-only output for one stereo frame. The dry blend happens in [`FxSlot`].
    fn process_frame(&mut self, left: f32, right: f32) -> (f32, f32);
}

#[derive(Debug)]
pub struct FxSlot {
    sample_rate: u32,
    kind: FxKind,
    enabled: bool,
    wet: f32,
    params: [f32; FX_PARAM_COUNT],
    fx: Option<Box<dyn Fx>>,
}

impl FxSlot {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            kind: FxKind::Off,
            enabled: false,
            wet: 0.5,
            params: [0.0; FX_PARAM_COUNT],
            fx: None,
        }
    }

    pub fn kind(&self) -> FxKind {
        self.kind
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn wet(&self) -> f32 {
        self.wet
    }

    pub fn params(&self) -> [f32; FX_PARAM_COUNT] {
        self.params
    }

    pub fn is_active(&self) -> bool {
        self.enabled && self.fx.is_some()
    }

    /// Select the effect, load its default knobs, drop the previous effect's state.
    /// Never enables the slot (a freshly selected effect stays off); `Off` forces
    /// `enabled = false`. Re-selecting the current kind is a no-op.
    pub fn set_kind(&mut self, kind: FxKind) {
        if kind == self.kind {
            return;
        }
        self.kind = kind;
        self.fx = kind.create(self.sample_rate);
        match self.fx.as_ref() {
            Some(fx) => self.params = fx.default_params(),
            None => {
                self.params = [0.0; FX_PARAM_COUNT];
                self.enabled = false;
            }
        }
    }

    /// Bypass switch; disabling clears the effect's state so a later enable starts clean.
    /// Enabling while `kind == Off` stays disabled.
    pub fn set_enabled(&mut self, enabled: bool) {
        if !enabled {
            self.reset();
        }
        self.enabled = enabled && self.fx.is_some();
    }

    /// Clear the effect's stored state (delay line, feedback, tails), keeping
    /// the kind, enable flag, wet, and knobs.
    pub fn reset(&mut self) {
        if let Some(fx) = self.fx.as_mut() {
            fx.reset();
        }
    }

    /// Wet/dry `0..1` (`0` = dry), clamped.
    pub fn set_wet(&mut self, wet: f32) {
        self.wet = wet.clamp(0.0, 1.0);
    }

    /// Knob `index`, normalized `0..1` (clamped).
    pub fn set_param(&mut self, index: usize, value: f32) -> anyhow::Result<()> {
        let fx = self
            .fx
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Invalid FX parameter: {index}"))?;
        if index >= fx.param_count() {
            return Err(anyhow::anyhow!("Invalid FX parameter: {index}"));
        }
        let value = value.clamp(0.0, 1.0);
        fx.set_param(index, value);
        self.params[index] = value;
        Ok(())
    }

    pub fn set_tempo(&mut self, bpm: f32) {
        if let Some(fx) = self.fx.as_mut() {
            fx.set_tempo(bpm);
        }
    }

    fn process_frame(&mut self, left: f32, right: f32) -> (f32, f32) {
        if !self.is_active() {
            return (left, right);
        }
        let wet = self.wet;
        let (wet_l, wet_r) = self
            .fx
            .as_mut()
            .expect("active slot has an effect")
            .process_frame(left, right);
        (
            left * (1.0 - wet) + wet_l * wet,
            right * (1.0 - wet) + wet_r * wet,
        )
    }
}

#[derive(Debug)]
pub struct FxChain {
    slots: [FxSlot; FX_SLOT_COUNT],
}

impl FxChain {
    /// A chain with the Beat FX unit loaded but unarmed: slot 0 holds
    /// [`FxKind::Echo`] because the FX SELECT has no "off" entry, so the unit
    /// always carries an effect. The remaining capacity stays empty for presets.
    pub fn new(sample_rate: u32) -> Self {
        let mut slots: [FxSlot; FX_SLOT_COUNT] = std::array::from_fn(|_| FxSlot::new(sample_rate));
        slots[0].set_kind(FxKind::Echo);
        Self { slots }
    }

    pub fn slots(&self) -> &[FxSlot; FX_SLOT_COUNT] {
        &self.slots
    }

    /// Clear every slot's stored effect state.
    ///
    /// Routing changes call this: an unrouted chain is not advanced, so its
    /// buffers would otherwise hold audio captured before the deck went dry.
    pub fn reset(&mut self) {
        for slot in &mut self.slots {
            slot.reset();
        }
    }

    pub fn slot(&self, index: usize) -> Option<&FxSlot> {
        self.slots.get(index)
    }

    pub fn slot_mut(&mut self, index: usize) -> Option<&mut FxSlot> {
        self.slots.get_mut(index)
    }

    /// Master-deck tempo for beat-synced effects (each effect keeps its own copy).
    pub fn set_tempo(&mut self, bpm: f32) {
        for slot in &mut self.slots {
            slot.set_tempo(bpm);
        }
    }

    /// In-place interleaved-stereo processing; returns immediately when no slot is active.
    pub fn process_buffer(&mut self, buffer: &mut [Sample]) {
        if !self.slots.iter().any(FxSlot::is_active) {
            return;
        }
        for frame in buffer.chunks_mut(2) {
            if frame.len() < 2 {
                break;
            }
            let (mut left, mut right) = (frame[0], frame[1]);
            for slot in &mut self.slots {
                let (next_l, next_r) = slot.process_frame(left, right);
                left = next_l;
                right = next_r;
            }
            frame[0] = left;
            frame[1] = right;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RATE: u32 = 48_000;

    /// One full-scale frame, then silence.
    fn impulse(frames: usize) -> Vec<Sample> {
        let mut buffer = vec![0.0; frames * 2];
        buffer[0] = 1.0;
        buffer[1] = 1.0;
        buffer
    }

    #[test]
    fn chain_without_an_active_slot_is_identity() {
        // A fresh chain has the unit loaded but unarmed.
        let mut chain = FxChain::new(SAMPLE_RATE);
        assert_eq!(chain.slot(0).expect("slot 0").kind(), FxKind::Echo);
        assert!(!chain.slot(0).expect("slot 0").enabled());

        let mut buffer = impulse(64);
        let before = buffer.clone();
        chain.process_buffer(&mut buffer);
        assert_eq!(
            buffer, before,
            "an inactive chain must not touch the buffer"
        );
    }

    #[test]
    fn wet_zero_passes_the_dry_signal_through() {
        let mut chain = FxChain::new(SAMPLE_RATE);
        {
            let slot = chain.slot_mut(0).expect("slot 0");
            slot.set_enabled(true);
            slot.set_wet(0.0);
        }

        let mut buffer = impulse(64);
        let before = buffer.clone();
        chain.process_buffer(&mut buffer);
        assert_eq!(buffer, before, "wet 0.0 must stay dry");
    }

    #[test]
    fn disabling_a_slot_drops_its_tail() {
        let mut chain = FxChain::new(SAMPLE_RATE);
        {
            let slot = chain.slot_mut(0).expect("slot 0");
            slot.set_enabled(true);
            slot.set_wet(1.0);
        }

        // Half a beat at the fallback tempo is 250 ms = 12000 frames.
        let mut wet = impulse(13_000);
        chain.process_buffer(&mut wet);
        assert!(
            wet.iter().any(|sample| *sample != 0.0),
            "the echo must ring into the delay line"
        );

        {
            let slot = chain.slot_mut(0).expect("slot 0");
            slot.set_enabled(false);
            slot.set_enabled(true);
        }
        let mut silence = vec![0.0; 13_000 * 2];
        chain.process_buffer(&mut silence);
        assert!(
            silence.iter().all(|sample| *sample == 0.0),
            "disabling must clear the stored tail"
        );
    }

    #[test]
    fn set_kind_loads_defaults_and_stays_unarmed() {
        let mut slot = FxSlot::new(SAMPLE_RATE);
        assert_eq!(slot.kind(), FxKind::Off);
        assert!(!slot.enabled());

        slot.set_kind(FxKind::Echo);
        assert_eq!(slot.kind(), FxKind::Echo);
        assert!(!slot.enabled(), "selecting an effect never arms the slot");
        assert_eq!(slot.params(), [0.5, 0.5, 0.0]);

        // Re-selecting the same kind keeps the tweaked knobs.
        slot.set_param(1, 0.9).expect("feedback knob");
        let tuned = slot.params();
        slot.set_kind(FxKind::Echo);
        assert_eq!(slot.params(), tuned);

        // Going Off clears the knobs, disarms and drops the effect.
        slot.set_enabled(true);
        slot.set_kind(FxKind::Off);
        assert_eq!(slot.kind(), FxKind::Off);
        assert!(!slot.enabled());
        assert_eq!(slot.params(), [0.0; FX_PARAM_COUNT]);
        assert!(slot.set_param(0, 0.5).is_err(), "no knobs while Off");
    }

    #[test]
    fn set_param_clamps_and_rejects_unknown_knobs() {
        let mut slot = FxSlot::new(SAMPLE_RATE);
        slot.set_kind(FxKind::Echo);

        slot.set_param(0, 5.0).expect("clamped write");
        assert_eq!(slot.params()[0], 1.0);
        slot.set_param(0, -3.0).expect("clamped write");
        assert_eq!(slot.params()[0], 0.0);
        // Echo exposes two knobs, so index 2 is out of range.
        assert!(slot.set_param(2, 0.5).is_err());
    }
}
