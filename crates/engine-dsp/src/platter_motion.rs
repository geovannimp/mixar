//! First-party platter motion authority.
//!
//! Live jog ticks, (future) UI position chase, and Pad FX rate envelopes all
//! resolve to one playback `rate`. Vinyl filtering uses a fixed 1 ms alpha–beta
//! clock (standard filter math; not a copy of Mixxx/xwax sources).

/// Platter policy for touched (top) or untouched (outer) turns.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MotionMode {
    #[default]
    Vinyl,
    PitchBend,
    Ignore,
}

const INTERVALS_PER_REV: f64 = 720.0;
const RPM: f64 = 100.0 / 3.0; // 33⅓
const ALPHA: f64 = 1.0 / 8.0;
const BETA: f64 = ALPHA / 32.0;
const QUANTUM_SECS: f64 = 0.001;
// Tuned to pre-PlatterMotion deck behavior: 20 ms vinyl idle hold, 8/s bend decay
// (from git history of the removed JOG_VINYL_HOLD_SECS / JOG_BEND_DECAY_PER_SEC).
const VINYL_HOLD_SECS: f64 = 0.02;
const RELEASE_RATE_PER_SEC: f32 = 12.0;
const BEND_PER_TICK: f32 = 0.0015;
const BEND_DECAY_PER_SEC: f32 = 8.0;
const BEND_MAX: f32 = 0.08;

/// Shared motion state for a deck platter.
#[derive(Debug)]
pub struct PlatterMotion {
    top: MotionMode,
    outer: MotionMode,
    touching: bool,
    rate: f32,
    /// Alpha–beta position residual (relative).
    filter_x: f64,
    /// Alpha–beta velocity in absolute music-rate units (`1.0` = vinyl speed).
    filter_v: f64,
    tick_accum: i32,
    idle_secs: f64,
    releasing: bool,
    /// Pad FX / scripted override; when set, wins over live filter.
    envelope_rate: Option<f32>,
    /// Fractional engine samples carried between 1 ms quanta.
    quantum_samples_rem: f64,
}

/// Idle/unity defaults for `Deck` initialization (same as [`PlatterMotion::new`]).
impl Default for PlatterMotion {
    fn default() -> Self {
        Self::new()
    }
}

impl PlatterMotion {
    pub fn new() -> Self {
        Self {
            top: MotionMode::Vinyl,
            outer: MotionMode::PitchBend,
            touching: false,
            rate: 1.0,
            filter_x: 0.0,
            filter_v: 0.0,
            tick_accum: 0,
            idle_secs: 1.0,
            releasing: false,
            envelope_rate: None,
            quantum_samples_rem: 0.0,
        }
    }

    pub fn set_modes(&mut self, top: MotionMode, outer: MotionMode, transport_playing: bool) {
        let was = self.active_mode(transport_playing);
        self.top = top;
        self.outer = outer;
        if was == MotionMode::Vinyl && self.active_mode(transport_playing) != MotionMode::Vinyl {
            self.begin_release(transport_playing);
        }
    }

    pub fn modes(&self) -> (MotionMode, MotionMode) {
        (self.top, self.outer)
    }

    /// Update touch. Does **not** handle slip exit — `Deck::set_jog_touch` owns
    /// slip catch-up + [`hard_reset_rate`] when leaving vinyl with slip enabled.
    ///
    /// `transport_playing` must match the deck transport at the call site.
    pub fn set_touch(&mut self, touching: bool, transport_playing: bool) {
        let was_touching = self.touching;
        let was = self.active_mode(transport_playing);
        self.touching = touching;
        let now = self.active_mode(transport_playing);

        // Pad FX owns absolute rate while an envelope is active — only track touch.
        if self.envelope_rate.is_some() {
            return;
        }

        // Finger lift always ends scratch authority. Paused outer remaps to
        // vinyl, so we cannot key only on was/now mode inequality.
        if was_touching && !touching {
            self.begin_release(transport_playing);
            return;
        }

        if was == MotionMode::Vinyl && now != MotionMode::Vinyl {
            self.begin_release(transport_playing);
            return;
        }

        if now == MotionMode::Vinyl && touching {
            self.releasing = false;
            self.idle_secs = 0.0;
            self.tick_accum = 0;
            self.filter_x = 0.0;
            self.filter_v = 0.0;
            self.rate = 0.0;
        }
    }

    pub fn add_ticks(&mut self, delta: i32) {
        if delta == 0 || self.envelope_rate.is_some() {
            return;
        }
        self.releasing = false;
        self.idle_secs = 0.0;
        self.tick_accum = self.tick_accum.saturating_add(delta);
    }

    /// Absolute music-rate override for Pad FX patterns.
    ///
    /// `None` clears the envelope and begins a release (paused → idle `1.0`
    /// immediately; playing → ramp), so a stale Pad FX rate cannot keep scrubbing.
    pub fn set_envelope_rate(&mut self, rate: Option<f32>, transport_playing: bool) {
        match rate {
            Some(r) => {
                let r = if r.is_finite() {
                    r.clamp(-8.0, 8.0)
                } else {
                    0.0
                };
                self.envelope_rate = Some(r);
                self.rate = r;
                self.releasing = false;
            }
            None => {
                if self.envelope_rate.take().is_some() {
                    // Touch may have been latched while the envelope owned rate;
                    // restore vinyl stop instead of releasing under a held finger.
                    if self.touching && self.active_mode(transport_playing) == MotionMode::Vinyl {
                        self.releasing = false;
                        self.idle_secs = 0.0;
                        self.tick_accum = 0;
                        self.filter_x = 0.0;
                        self.filter_v = 0.0;
                        self.rate = 0.0;
                    } else {
                        self.begin_release(transport_playing);
                    }
                }
            }
        }
    }

    /// Advance motion by `frames` at `sample_rate`.
    ///
    /// `transport_playing` must match the deck transport for this callback.
    pub fn tick(&mut self, frames: usize, sample_rate: u32, transport_playing: bool) {
        let sample_rate = sample_rate.max(1);
        let dt_buf = frames as f64 / f64::from(sample_rate);
        self.idle_secs += dt_buf;

        if let Some(r) = self.envelope_rate {
            self.rate = r;
            return;
        }

        let mode = self.active_mode(transport_playing);

        if self.releasing {
            self.ramp_release(dt_buf, transport_playing);
            if mode == MotionMode::Vinyl {
                self.drain_vinyl_quanta(frames, sample_rate, false);
            }
            return;
        }

        match mode {
            MotionMode::Vinyl => {
                self.drain_vinyl_quanta(frames, sample_rate, true);
                if self.idle_secs >= VINYL_HOLD_SECS {
                    if self.touching {
                        self.filter_x = 0.0;
                        self.filter_v = 0.0;
                        self.rate = 0.0;
                    } else if self.filter_v.abs() > 1e-4 {
                        // Residual throw / scrub coast.
                        self.filter_v *= 0.5_f64.powf(dt_buf / 0.01);
                        if self.filter_v.abs() < 1e-4 {
                            self.filter_v = 0.0;
                            self.filter_x = 0.0;
                            // Coast done → re-engage motor (1.0), playing or paused.
                            self.rate = 1.0;
                        } else {
                            self.rate = self.filter_v as f32;
                        }
                    } else {
                        // Idle free platter: motor at unity (not stopped at 0).
                        self.filter_x = 0.0;
                        self.rate = 1.0;
                    }
                }
            }
            MotionMode::PitchBend => {
                if self.tick_accum != 0 {
                    let bend = (self.rate - 1.0) + (self.tick_accum as f32) * BEND_PER_TICK;
                    self.rate = 1.0 + bend.clamp(-BEND_MAX, BEND_MAX);
                    self.tick_accum = 0;
                    self.idle_secs = 0.0;
                }
                if self.idle_secs > 0.02 {
                    let decay = (-BEND_DECAY_PER_SEC * (dt_buf as f32)).exp();
                    let bend = (self.rate - 1.0) * decay;
                    if bend.abs() < 1e-4 {
                        self.rate = 1.0;
                    } else {
                        self.rate = 1.0 + bend;
                    }
                }
            }
            MotionMode::Ignore => {
                self.tick_accum = 0;
                if !self.releasing {
                    self.rate = 1.0;
                }
            }
        }
    }

    pub fn rate(&self) -> f32 {
        self.rate
    }

    pub fn touching(&self) -> bool {
        self.touching
    }

    pub fn driving(&self) -> bool {
        // Release ramp still owns the motor until it settles.
        if self.releasing {
            return true;
        }
        // Ignore top: touch is a no-op; don't force paused audio.
        if self.touching && self.top != MotionMode::Ignore {
            return true;
        }
        if let Some(r) = self.envelope_rate {
            // Envelope at exactly 1.0 is a no-op motor; don't force audio.
            return r.is_finite() && (r - 1.0).abs() > 1e-4;
        }
        // Rate away from the idle motor (1.0). Near-zero after a paused coast is
        // "stopped", not an active scrub (avoids outputting a held sample).
        let r = self.rate;
        r.is_finite() && (r - 1.0).abs() > 1e-4 && r.abs() > 1e-4
    }

    /// Resolve top/outer policy. Paused outer pitch-bend remaps to vinyl scrub.
    pub fn active_mode(&self, transport_playing: bool) -> MotionMode {
        let base = if self.touching { self.top } else { self.outer };
        if !transport_playing && base == MotionMode::PitchBend {
            MotionMode::Vinyl
        } else {
            base
        }
    }

    pub fn begin_release(&mut self, transport_playing: bool) {
        // Pad FX owns rate while an envelope is active.
        if self.envelope_rate.is_some() {
            return;
        }
        self.tick_accum = 0;
        self.filter_x = 0.0;
        self.filter_v = 0.0;
        if !transport_playing {
            self.releasing = false;
            self.rate = 1.0;
            return;
        }
        self.releasing = true;
    }

    /// Slip exit: hard-clear live vinyl/bend without a release ramp.
    ///
    /// Does **not** clear an active Pad FX envelope — that ownership stays with
    /// the envelope caller until `set_envelope_rate(None, …)`.
    pub fn hard_reset_rate(&mut self) {
        self.tick_accum = 0;
        self.filter_x = 0.0;
        self.filter_v = 0.0;
        self.releasing = false;
        if let Some(r) = self.envelope_rate {
            self.rate = r;
        } else {
            self.rate = 1.0;
        }
    }

    fn ramp_release(&mut self, dt: f64, transport_playing: bool) {
        if !transport_playing {
            self.releasing = false;
            self.rate = 1.0;
            return;
        }
        let step = RELEASE_RATE_PER_SEC * (dt as f32);
        if (self.rate - 1.0).abs() <= step {
            self.rate = 1.0;
            self.releasing = false;
        } else {
            self.rate += (1.0 - self.rate).signum() * step;
        }
    }

    fn drain_vinyl_quanta(&mut self, frames: usize, sample_rate: u32, apply_ticks: bool) {
        let samples_per_quantum = f64::from(sample_rate) * QUANTUM_SECS;
        if samples_per_quantum <= 0.0 {
            return;
        }
        let mut samples = frames as f64 + self.quantum_samples_rem;
        let quantum_count = (samples / samples_per_quantum).floor() as i32;
        if quantum_count <= 0 {
            self.quantum_samples_rem = samples;
            return;
        }

        // Spread buffer-accumulated ticks across the 1 ms quanta so a 512-frame
        // callback (~10 ms) does not dump the whole accumulator into one step.
        let total_ticks = if apply_ticks {
            std::mem::take(&mut self.tick_accum)
        } else {
            0
        };
        let base = total_ticks / quantum_count;
        let mut rem = total_ticks % quantum_count;

        for _ in 0..quantum_count {
            samples -= samples_per_quantum;
            let mut q = base;
            if rem != 0 {
                let step = if rem > 0 { 1 } else { -1 };
                q += step;
                rem -= step;
            }
            self.observe_vinyl(q);
        }
        self.quantum_samples_rem = samples;
    }

    fn observe_vinyl(&mut self, ticks: i32) {
        let intervals_per_sec = RPM * INTERVALS_PER_REV / 60.0;
        let dx = f64::from(ticks) / intervals_per_sec;
        let predicted_x = self.filter_x + self.filter_v * QUANTUM_SECS;
        let residual = dx - predicted_x;
        self.filter_x = predicted_x + residual * ALPHA;
        self.filter_v += residual * BETA / QUANTUM_SECS;
        self.filter_x -= dx;
        self.rate = (self.filter_v as f32).clamp(-3.0, 3.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paused_touch_idles_at_zero_and_drives() {
        let mut m = PlatterMotion::new();
        m.set_touch(true, false);
        assert_eq!(m.rate(), 0.0);
        assert!(m.driving());
        m.tick(512, 48_000, false);
        assert_eq!(m.rate(), 0.0);
        assert!(m.driving());
    }

    #[test]
    fn steady_ticks_near_vinyl_speed_yield_near_unity_rate() {
        let mut m = PlatterMotion::new();
        m.set_touch(true, false);
        // Vinyl speed ≈ 400 ticks/s → feed 2 ticks every 5 ms.
        for _ in 0..200 {
            m.add_ticks(2);
            m.tick(240, 48_000, false);
        }
        let r = m.rate();
        assert!(
            (r - 1.0).abs() < 0.35,
            "expected ~1.0 vinyl rate after settle, got {r}"
        );
    }

    #[test]
    fn faster_ticks_raise_rate_above_unity() {
        let mut m = PlatterMotion::new();
        m.set_touch(true, false);
        for _ in 0..200 {
            m.add_ticks(2);
            m.tick(48, 48_000, false);
        }
        assert!(m.rate() > 1.5, "rate={}", m.rate());
    }

    #[test]
    fn envelope_overrides_live_ticks() {
        let mut m = PlatterMotion::new();
        m.set_touch(true, false);
        m.set_envelope_rate(Some(-1.5), false);
        m.add_ticks(50);
        m.tick(512, 48_000, false);
        assert!((m.rate() - (-1.5)).abs() < 1e-4);
        assert!(m.driving());
        m.set_envelope_rate(None, false);
        m.set_touch(true, false);
        assert_eq!(m.rate(), 0.0);
    }

    #[test]
    fn paused_outer_ticks_scrub_not_pitch_bend() {
        let mut m = PlatterMotion::new();
        assert!(!m.touching());
        for _ in 0..100 {
            m.add_ticks(2);
            m.tick(48, 48_000, false);
        }
        let r = m.rate().abs();
        assert!(
            r > 0.2,
            "paused outer must scrub with absolute rate, got {r}"
        );
    }

    #[test]
    fn playing_outer_pitch_bend_decays_to_unity() {
        let mut m = PlatterMotion::new();
        m.add_ticks(40);
        m.tick(512, 48_000, true);
        assert!((m.rate() - 1.0).abs() > 1e-3, "rate={}", m.rate());
        for _ in 0..100 {
            m.tick(512, 48_000, true);
        }
        assert!((m.rate() - 1.0).abs() < 1e-3, "rate={}", m.rate());
        assert!(!m.driving());
    }

    #[test]
    fn paused_release_does_not_keep_driving() {
        let mut m = PlatterMotion::new();
        m.set_touch(true, false);
        m.add_ticks(10);
        m.tick(48, 48_000, false);
        m.set_touch(false, false);
        assert!(!m.driving(), "rate={}", m.rate());
        assert!((m.rate() - 1.0).abs() < 1e-4);
    }

    #[test]
    fn playing_idle_vinyl_outer_keeps_motor_at_unity() {
        let mut m = PlatterMotion::new();
        m.set_modes(MotionMode::Vinyl, MotionMode::Vinyl, true);
        assert!(!m.touching());
        for _ in 0..10 {
            m.tick(512, 48_000, true);
        }
        assert!(
            (m.rate() - 1.0).abs() < 1e-4,
            "idle playing vinyl must not stop motor, rate={}",
            m.rate()
        );
        assert!(!m.driving());
    }

    #[test]
    fn idle_free_platter_zeros_filter_x() {
        let mut m = PlatterMotion::new();
        m.set_modes(MotionMode::Vinyl, MotionMode::Vinyl, true);
        // Settled velocity with a stale position residual. frames=0 skips
        // observe_vinyl so we hit the idle free-platter branch directly.
        m.filter_x = 0.25;
        m.filter_v = 0.0;
        m.rate = 1.0;
        m.idle_secs = VINYL_HOLD_SECS;
        m.tick(0, 48_000, true);
        assert!(
            (m.rate() - 1.0).abs() < 1e-4,
            "idle-branch rate expected 1.0, got {} (filter_x={}, filter_v={})",
            m.rate(),
            m.filter_x,
            m.filter_v
        );
        assert!(
            m.filter_x.abs() < 1e-12,
            "idle free platter must clear stale filter_x, got {}",
            m.filter_x
        );
    }

    #[test]
    fn envelope_at_unity_does_not_drive() {
        let mut m = PlatterMotion::new();
        m.set_envelope_rate(Some(1.0), false);
        assert!(!m.driving());
        m.set_envelope_rate(Some(0.0), false);
        assert!(m.driving());
    }

    #[test]
    fn ignore_touch_does_not_drive_while_paused() {
        let mut m = PlatterMotion::new();
        m.set_modes(MotionMode::Ignore, MotionMode::Ignore, false);
        m.set_touch(true, false);
        m.tick(512, 48_000, false);
        assert!((m.rate() - 1.0).abs() < 1e-4, "rate={}", m.rate());
        assert!(
            !m.driving(),
            "Ignore touch must not force paused audio, rate={}",
            m.rate()
        );
    }

    #[test]
    fn touch_with_active_envelope_keeps_envelope_rate() {
        let mut m = PlatterMotion::new();
        m.set_envelope_rate(Some(-1.5), false);
        m.set_touch(true, false);
        assert!(
            (m.rate() - (-1.5)).abs() < 1e-4,
            "touch must not zero through an active envelope, rate={}",
            m.rate()
        );
        assert!(m.touching());
    }

    #[test]
    fn begin_release_preserves_active_envelope_rate() {
        let mut m = PlatterMotion::new();
        m.set_envelope_rate(Some(2.0), true);
        m.set_touch(true, true);
        m.begin_release(true);
        assert!(
            (m.rate() - 2.0).abs() < 1e-4,
            "begin_release must not stomp envelope, rate={}",
            m.rate()
        );
        assert!(!m.releasing);
        m.tick(512, 48_000, true);
        assert!((m.rate() - 2.0).abs() < 1e-4);
    }

    #[test]
    fn clearing_envelope_while_touching_vinyl_holds_at_zero() {
        let mut m = PlatterMotion::new();
        m.set_modes(MotionMode::Vinyl, MotionMode::Vinyl, false);
        m.set_envelope_rate(Some(2.0), false);
        m.set_touch(true, false);
        assert!(m.touching());
        m.set_envelope_rate(None, false);
        assert!(
            (m.rate() - 0.0).abs() < 1e-4,
            "cleared envelope under vinyl touch must hold stop, rate={}",
            m.rate()
        );
        assert!(!m.releasing);
        assert!(m.driving());
    }

    #[test]
    fn clearing_envelope_releases_stale_rate_when_paused() {
        let mut m = PlatterMotion::new();
        m.set_envelope_rate(Some(2.0), false);
        assert!(m.driving());
        m.set_envelope_rate(None, false);
        assert!(
            (m.rate() - 1.0).abs() < 1e-4,
            "paused envelope clear must idle at 1.0, rate={}",
            m.rate()
        );
        assert!(!m.driving());
    }

    #[test]
    fn hard_reset_preserves_active_envelope() {
        let mut m = PlatterMotion::new();
        m.set_envelope_rate(Some(-1.25), true);
        m.hard_reset_rate();
        assert!((m.rate() - (-1.25)).abs() < 1e-4);
        assert!(m.driving());
    }

    #[test]
    fn long_buffer_distributes_ticks_without_max_spike() {
        let mut m = PlatterMotion::new();
        m.set_touch(true, false);
        // Warm near vinyl: 2 ticks / 5 ms ≈ 400 ticks/s.
        for _ in 0..200 {
            m.add_ticks(2);
            m.tick(240, 48_000, false);
        }
        let warm = m.rate().abs();
        assert!(warm > 0.5 && warm < 1.5, "warm rate={warm}");
        // 20 ticks in one 512-frame buffer: dump-into-first-1ms would slam to ±3.
        // Spread across ~10 quanta stays near a few× vinyl, not the clamp.
        m.add_ticks(20);
        m.tick(512, 48_000, false);
        let after = m.rate().abs();
        assert!(
            after < 2.9,
            "ticks must be spread across quanta; spiked rate={after} (warm={warm})"
        );
    }
}
