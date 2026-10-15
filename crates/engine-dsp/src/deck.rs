//! Audio deck implementation for DJ-style playback
//!
//! A deck represents a single audio playback unit with transport and
//! playback-speed controls.

use crate::platter_motion::{MotionMode, PlatterMotion};
use crate::transport::DeckTransportEvent;
use anyhow::Result;
use audio_core::{secs_to_ms, LoadedAudio, Sample};
use dasp_graph::Buffer;
use resampler::Resampler;
use std::fmt;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use stretch::{create_stretcher, TimeStretcher, KEY_SHIFT_SEMITONE_LIMIT};

/// Audio deck state
#[derive(Debug, Clone, PartialEq)]
pub enum DeckState {
    /// Deck is stopped
    Stopped,
    /// Deck is playing
    Playing,
    /// Deck is paused
    Paused,
}

/// Platter policy for touched (top) or untouched (outer) jog turns.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum JogMode {
    #[default]
    Vinyl,
    PitchBend,
    Ignore,
}

fn jog_to_motion(mode: JogMode) -> MotionMode {
    match mode {
        JogMode::Vinyl => MotionMode::Vinyl,
        JogMode::PitchBend => MotionMode::PitchBend,
        JogMode::Ignore => MotionMode::Ignore,
    }
}

fn motion_to_jog(mode: MotionMode) -> JogMode {
    match mode {
        MotionMode::Vinyl => JogMode::Vinyl,
        MotionMode::PitchBend => JogMode::PitchBend,
        MotionMode::Ignore => JogMode::Ignore,
    }
}

/// Active vinyl brake ramp (pause spin-down).
#[derive(Clone, Copy, Debug)]
struct VinylBrake {
    /// Motor rate captured when pause started.
    start_rate: f32,
    /// Frames processed since brake start.
    elapsed_frames: usize,
    /// Brake length in frames (>= 1).
    total_frames: usize,
}

/// Parameters for building a [`PreparedStretcher`] outside the DSP lock.
pub struct StretcherSpec {
    sample_rate: u32,
    buffer_size: u32,
}

impl StretcherSpec {
    /// Construct the realtime stretcher. Heavy: call off the DSP lock.
    pub fn build(&self) -> Result<PreparedStretcher> {
        Ok(PreparedStretcher(new_stretcher(
            self.sample_rate,
            self.buffer_size,
        )?))
    }
}

/// A fully constructed stretcher ready to install into a [`Deck`].
pub struct PreparedStretcher(Box<dyn TimeStretcher>);

/// Audio deck for DJ-style playback
pub struct Deck {
    /// Deck identifier
    id: usize,
    /// Current state
    state: DeckState,
    /// Current position in source frames (integer floor; may be negative).
    position_frames: i64,
    /// Sub-frame source position for tempo / loop accuracy (signed; source of truth).
    position_frac: f64,
    /// Tempo fader position `0..1` (center = track BPM).
    speed: f32,
    /// Half-span of the tempo fader in BPM (default ±8).
    tempo_range: f32,
    /// Track BPM for fader→ratio; `None` uses a fallback inside ratio math.
    track_bpm: Option<f64>,
    /// When set (tempo sync), DSP uses this ratio instead of fader-derived.
    ratio_override: Option<f32>,
    /// Vinyl / bend / envelope motion authority (playback-rate multiplier).
    motion: PlatterMotion,
    /// Immutable engine output sample rate (from config).
    sample_rate: u32,
    /// Immutable engine callback size in frames (from config).
    buffer_size: u32,
    /// Internal buffer for audio processing
    buffer: Vec<Sample>,
    /// Whether the deck is currently processing audio
    processing: bool,
    /// Shared decoded audio (cache and multiple decks can reference the same buffer).
    loaded: Option<Arc<LoadedAudio>>,
    /// Optional four stem layers (drums, bass, other, vocals — NI index order), same playhead as `loaded`.
    stems: Option<[Arc<LoadedAudio>; 4]>,
    /// Per-stem mute (true = silent).
    stem_mute: [bool; 4],
    /// When set, only this stem index is audible.
    stem_isolate: Option<u8>,
    /// Smoothed gains applied in the mix (ramps toward mute/isolate targets).
    stem_gain_current: [f32; 4],
    /// Resampler quality preset (`low`, `medium`, or `high`).
    resampler_quality: String,
    /// Resampler for converting between sample rates (created on load when needed)
    resampler: Option<Box<dyn Resampler>>,
    /// Key lock: tempo via time-stretch, pitch held (inactive during vinyl jog).
    key_lock: bool,
    /// Vinyl brake on Pause: spin down over `vinyl_brake_stop_secs` when true.
    vinyl_brake_enabled: bool,
    /// Brake stop duration in seconds (clamped to 0.05..=3.0).
    vinyl_brake_stop_secs: f32,
    /// Active brake ramp; `None` when not braking.
    brake: Option<VinylBrake>,
    /// timestretch stretcher at engine rate (created lazily / on load).
    stretcher: Option<Box<dyn TimeStretcher>>,
    /// Last process used stretch path (reset stretcher when leaving it).
    stretch_active: bool,
    /// Set once a stretched process fell back to vinyl for a missing stretcher
    /// during the current stretch episode, so the audio thread logs the failure
    /// once per transition instead of once per callback.
    stretcher_fallback_logged: bool,
    /// Session key-shift offset in semitones (`-16..=16`; `0` = bypass resampler).
    key_shift_semitones: f32,
    /// Active loop region in source frames (inclusive start, exclusive end).
    loop_region: Option<(f64, f64)>,
    /// Slip mode: shadow playhead keeps advancing while audible pos is looped/scratched.
    slip_enabled: bool,
    /// Slip cursor source position (frames, fractional). `Some` only while a
    /// manual cursor action (jog scratch / loop / cue hold) is active; `None`
    /// means no cursor — nothing is calculated or published.
    shadow_position_frac: Option<f64>,
    /// Temporary cue point in milliseconds (source time; may be negative).
    cue_point_ms: Option<i32>,
    /// Saved transport state while cue is held (return position ms, was_playing).
    cue_hold_return: Option<(i32, bool)>,
    /// Events to deliver to the engine notifier after processing.
    pending_transport: Vec<DeckTransportEvent>,
}

impl fmt::Debug for Deck {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Deck")
            .field("id", &self.id)
            .field("state", &self.state)
            .field("position_frames", &self.position_frames)
            .field("speed", &self.speed)
            .field("sample_rate", &self.sample_rate)
            .field("buffer_size", &self.buffer_size)
            .field("processing", &self.processing)
            .field(
                "loaded",
                &self.loaded.as_ref().map(|audio| audio.samples.len()),
            )
            .field(
                "source_id",
                &self.loaded.as_ref().map(|audio| audio.source_id.as_str()),
            )
            .field("resampler", &"<resampler>")
            .finish()
    }
}

impl Deck {
    /// Create a new deck with an immutable output clock from engine config.
    pub fn new(id: usize, sample_rate: u32, buffer_size: u32, resampler_quality: &str) -> Self {
        Self {
            id,
            state: DeckState::Stopped,
            position_frames: 0,
            position_frac: 0.0,
            speed: 0.5,
            tempo_range: crate::tempo::DEFAULT_TEMPO_RANGE,
            track_bpm: None,
            ratio_override: None,
            motion: PlatterMotion::new(),
            sample_rate,
            buffer_size: buffer_size.max(1),
            buffer: Vec::new(),
            processing: false,
            loaded: None,
            stems: None,
            stem_mute: [false; 4],
            stem_isolate: None,
            stem_gain_current: [1.0; 4],
            resampler_quality: resampler_quality.to_string(),
            resampler: None,
            key_lock: false,
            stretcher: None,
            stretch_active: false,
            vinyl_brake_enabled: false,
            vinyl_brake_stop_secs: 0.5,
            brake: None,
            stretcher_fallback_logged: false,
            key_shift_semitones: 0.0,
            loop_region: None,
            slip_enabled: false,
            shadow_position_frac: None,
            cue_point_ms: None,
            cue_hold_return: None,
            pending_transport: Vec::new(),
        }
    }

    /// Take pending transport events produced during the last process cycle.
    pub fn drain_transport_events(&mut self) -> Vec<DeckTransportEvent> {
        std::mem::take(&mut self.pending_transport)
    }

    fn check_track_end(&mut self) {
        if self.state != DeckState::Playing || self.loop_region.is_some() {
            return;
        }
        let Some(audio) = self.loaded.as_ref() else {
            return;
        };
        let total_frames = audio.samples.len() / 2;
        if total_frames == 0 {
            return;
        }
        if self.audible_position_frac() >= total_frames as f64 {
            self.state = DeckState::Paused;
            self.pending_transport.push(DeckTransportEvent::TrackEnded);
        }
    }

    /// Get the deck ID
    pub fn id(&self) -> usize {
        self.id
    }

    /// Get the current state
    pub fn state(&self) -> &DeckState {
        &self.state
    }

    /// Source frames still queued in the stretcher (not yet heard), plus
    /// pipeline delay converted to source frames.
    fn stretch_position_lead_frames(&self) -> f64 {
        if !self.stretch_active {
            return 0.0;
        }
        let Some(stretcher) = self.stretcher.as_ref() else {
            return 0.0;
        };
        let queued = stretcher.queued_source_frames() as f64;
        let delay_out = stretcher.start_delay() as f64;
        if delay_out <= 0.0 {
            return queued;
        }
        let Some(audio) = self.loaded.as_ref() else {
            return queued;
        };
        let src_per_out = (f64::from(audio.sample_rate) / f64::from(self.sample_rate))
            * f64::from(self.playback_ratio().max(0.01));
        queued + delay_out * src_per_out
    }

    /// Playhead for UI / end-of-track: feed cursor minus stretch lead.
    fn audible_position_frac(&self) -> f64 {
        self.position_frac - self.stretch_position_lead_frames()
    }

    /// Get the current position in source frames (may be negative).
    pub fn position_frames(&self) -> i64 {
        self.audible_position_frac().floor() as i64
    }

    /// Current playback position in milliseconds (source file time).
    pub fn position_ms(&self) -> Option<i32> {
        let audio = self.loaded.as_ref()?;
        Some(secs_to_ms(
            self.audible_position_frac() / f64::from(audio.sample_rate),
        ))
    }

    /// Temporary cue point in milliseconds, if set.
    pub fn cue_point_ms(&self) -> Option<i32> {
        self.cue_point_ms
    }

    /// Active loop region in milliseconds, if any.
    pub fn loop_region_ms(&self) -> Option<(i32, i32)> {
        let audio = self.loaded.as_ref()?;
        let rate = f64::from(audio.sample_rate);
        self.loop_region.map(|(start_frames, end_frames)| {
            (
                secs_to_ms(start_frames / rate),
                secs_to_ms(end_frames / rate),
            )
        })
    }

    /// Tempo fader position `0..1`.
    pub fn speed(&self) -> f32 {
        self.speed
    }

    /// Tempo fader half-span as pitch fraction (e.g. `0.06` = ±6%).
    pub fn tempo_range(&self) -> f32 {
        self.tempo_range
    }

    pub fn set_tempo_range(&mut self, range: f32) {
        self.tempo_range = range.max(0.0);
        // Keep fader position; if synced, remap displayed speed from override.
        if let Some(ratio) = self.ratio_override {
            self.speed = crate::tempo::playback_ratio_to_norm(ratio, self.tempo_range);
        }
    }

    pub fn set_track_bpm(&mut self, bpm: Option<f64>) {
        self.track_bpm = bpm.filter(|b| b.is_finite() && *b > 0.0);
        // Load clears sync ownership; fader position drives ratio again.
        self.ratio_override = None;
    }

    /// Playback ratio used by the resampler (1.0 = normal), before jog.
    pub fn playback_ratio(&self) -> f32 {
        if let Some(ratio) = self.ratio_override {
            return ratio.max(0.01);
        }
        crate::tempo::norm_to_playback_ratio(self.speed, self.tempo_range)
    }

    /// Set tempo fader position `0..1` (clears sync ratio override).
    pub fn set_speed(&mut self, speed: f32) -> Result<()> {
        if !(0.0..=1.0).contains(&speed) {
            return Err(anyhow::anyhow!("Speed must be between 0.0 and 1.0"));
        }
        self.speed = speed;
        self.ratio_override = None;
        Ok(())
    }

    /// Set absolute playback ratio (tempo sync). Updates fader position (clamped) for UI.
    pub fn set_playback_ratio(&mut self, ratio: f32) -> Result<()> {
        if ratio <= 0.0 {
            return Err(anyhow::anyhow!("Playback ratio must be positive"));
        }
        self.ratio_override = Some(ratio);
        self.speed = crate::tempo::playback_ratio_to_norm(ratio, self.tempo_range);
        Ok(())
    }

    /// Start playback
    pub fn play(&mut self) -> Result<()> {
        tracing::info!(
            "Deck {} changing state to Playing (was: {:?})",
            self.id,
            self.state
        );
        self.cancel_brake();
        self.state = DeckState::Playing;
        Ok(())
    }

    /// Pause playback
    pub fn pause(&mut self) -> Result<()> {
        if self.state == DeckState::Playing && self.vinyl_brake_enabled && !self.motion.touching() {
            self.start_brake();
        }
        self.state = DeckState::Paused;
        Ok(())
    }

    /// Begin a vinyl spin-down from the current motor rate over
    /// `vinyl_brake_stop_secs`. Callers supply the state/touch guards.
    fn start_brake(&mut self) {
        // The ramp advances once per audio block, so a window shorter than two
        // blocks would complete on the first tick and mute instantly instead of
        // spinning down.
        let block = self.buffer_size.max(1) as usize;
        let total = ((f64::from(self.vinyl_brake_stop_secs) * f64::from(self.sample_rate)).round()
            as usize)
            .max(1)
            .max(block * 2);
        let start_rate = self.motion.rate();
        self.brake = Some(VinylBrake {
            start_rate,
            elapsed_frames: 0,
            total_frames: total,
        });
        self.motion.set_envelope_rate(Some(start_rate), true);
    }

    /// Stop playback and reset position
    pub fn stop(&mut self) -> Result<()> {
        self.cancel_brake();
        self.state = DeckState::Stopped;
        self.position_frames = 0;
        self.position_frac = 0.0;
        self.cue_hold_return = None;
        self.reset_resampler_state();
        self.reset_stretcher_state();
        self.stretch_active = false;
        Ok(())
    }

    /// Key lock enabled (tempo stretch; ignored while vinyl jog drives audio).
    pub fn key_lock(&self) -> bool {
        self.key_lock
    }

    /// Configure pause brake. Disabling cancels any active brake.
    /// Non-finite `stop_secs` falls back to 0.5; finite values clamp to 0.05..=3.0.
    pub fn set_vinyl_brake(&mut self, enabled: bool, stop_secs: f32) {
        self.vinyl_brake_enabled = enabled;
        self.vinyl_brake_stop_secs = if stop_secs.is_finite() {
            stop_secs.clamp(0.05, 3.0)
        } else {
            0.5
        };
        if !enabled {
            self.cancel_brake();
        }
    }

    /// Enable/disable key lock. Resets stretcher state on change.
    pub fn set_key_lock(&mut self, enabled: bool) -> Result<()> {
        if self.key_lock == enabled {
            return Ok(());
        }
        self.key_lock = enabled;
        if enabled {
            self.ensure_stretcher()?;
        }
        self.reset_stretcher_state();
        self.stretch_active = false;
        Ok(())
    }

    /// Current key-shift offset in semitones (`-16..=16`; `0` = bypass).
    pub fn key_shift_semitones(&self) -> f32 {
        self.key_shift_semitones
    }

    /// Pitch multiplier for the current key-shift offset (`1.0` = unshifted).
    pub fn pitch_factor(&self) -> f64 {
        stretch::semitones_to_pitch(self.key_shift_semitones)
    }

    /// Set the key-shift offset in semitones: clamps to
    /// [`KEY_SHIFT_SEMITONE_LIMIT`] (`±16`, shared with the engine control layer;
    /// covers the pentatonic table's `+16`), non-finite → 0, and resets stretcher
    /// state so the new pitch takes effect cleanly.
    ///
    /// The bound is independent of the stretcher's
    /// [`MIN_PITCH_FACTOR`](stretch::MIN_PITCH_FACTOR)..=
    /// [`MAX_PITCH_FACTOR`](stretch::MAX_PITCH_FACTOR) range, but the constant is
    /// chosen so the worst-case pitch factor stays inside it — see
    /// [`KEY_SHIFT_SEMITONE_LIMIT`].
    pub fn set_key_shift_semitones(&mut self, semitones: f32) -> Result<()> {
        let s = if semitones.is_finite() {
            semitones.clamp(-KEY_SHIFT_SEMITONE_LIMIT, KEY_SHIFT_SEMITONE_LIMIT)
        } else {
            0.0
        };
        if (s - self.key_shift_semitones).abs() < f32::EPSILON {
            return Ok(());
        }
        if s != 0.0 {
            self.ensure_stretcher()?;
        }
        self.key_shift_semitones = s;
        self.reset_stretcher_state();
        self.stretch_active = false;
        Ok(())
    }

    pub fn top_jog_mode(&self) -> JogMode {
        motion_to_jog(self.motion.modes().0)
    }

    pub fn outer_jog_mode(&self) -> JogMode {
        motion_to_jog(self.motion.modes().1)
    }

    pub fn jog_touching(&self) -> bool {
        self.motion.touching()
    }

    pub fn jog_rate(&self) -> f32 {
        self.motion.rate()
    }

    /// Optional Pad FX / scripted absolute rate (`None` clears with release).
    pub fn set_motion_envelope_rate(&mut self, rate: Option<f32>) {
        self.brake = None;
        let playing = self.state == DeckState::Playing;
        self.motion.set_envelope_rate(rate, playing);
    }

    pub fn set_jog_mode(&mut self, top: JogMode, outer: JogMode) {
        let playing = self.state == DeckState::Playing;
        self.motion
            .set_modes(jog_to_motion(top), jog_to_motion(outer), playing);
        // Only a switch that actually leaves vinyl ends the scratch: `set_modes`
        // starts the release ramp in exactly that case. A Vinyl→Vinyl switch (or
        // a paused outer PitchBend remap) keeps the finger on a running platter,
        // so a touch-armed cursor must survive to catch up on the real release.
        // An in-flight loop or cue hold always keeps its cursor.
        let mode = self.motion.active_mode(playing);
        if self.motion.touching()
            && mode != MotionMode::Vinyl
            && self.loop_region.is_none()
            && self.cue_hold_return.is_none()
        {
            self.shadow_position_frac = None;
        }
    }

    pub fn set_jog_touch(&mut self, touching: bool) {
        if touching {
            self.cancel_brake();
        }
        let playing = self.state == DeckState::Playing;
        let was = self.motion.active_mode(playing);
        if was == MotionMode::Vinyl && !touching && self.motion.touching() && self.slip_enabled {
            // Leaving vinyl with slip: catch up and hard-clear (no release scrub).
            self.motion.set_touch(false, playing);
            self.catch_up_slip();
            self.motion.hard_reset_rate();
            self.shadow_position_frac = None;
            self.rearm_slip_shadow();
            return;
        }
        self.motion.set_touch(touching, playing);
        if touching
            && playing
            && self.slip_enabled
            && self.shadow_position_frac.is_none()
            && self.motion.active_mode(playing) == MotionMode::Vinyl
        {
            // Slip cursor arms at scratch start; idle playback keeps none.
            self.shadow_position_frac = Some(self.position_frac);
        }
        // Lifting the finger off a deck paused under brake must still spin the
        // platter down; otherwise the pause that happened while holding the jog
        // cuts instantly on release.
        if !touching
            && self.state == DeckState::Paused
            && self.vinyl_brake_enabled
            && self.brake.is_none()
        {
            self.start_brake();
        }
    }

    pub fn jog_turn(&mut self, delta: i32) {
        if delta == 0 {
            return;
        }
        let playing = self.state == DeckState::Playing;
        if self.motion.active_mode(playing) == MotionMode::Ignore {
            return;
        }
        self.motion.add_ticks(delta);
    }

    /// Advance platter filters for one output buffer (`frames` at engine sample rate).
    fn tick_jog(&mut self, frames: usize) {
        let playing = self.state == DeckState::Playing;
        self.motion.tick(frames, self.sample_rate, playing);
    }

    /// Drop an active vinyl brake and restore the idle motor (no release ramp).
    fn cancel_brake(&mut self) {
        if self.brake.take().is_some() {
            let playing = self.state == DeckState::Playing;
            self.motion.set_envelope_rate(None, playing);
            // Only a live transport must snap to the idle motor. On a paused deck
            // the vinyl filter holds the coast (used when a finger lifts), so
            // wiping it would throw away the scratch velocity.
            if playing {
                self.motion.hard_reset_rate();
            }
        }
    }

    /// Ramp the envelope rate linearly to zero over the brake window.
    /// Ends by clearing the envelope so `PlatterMotion::driving()` goes false.
    fn advance_brake(&mut self, frames: usize) {
        let Some(mut b) = self.brake else {
            return;
        };
        b.elapsed_frames = b.elapsed_frames.saturating_add(frames);
        if b.elapsed_frames >= b.total_frames {
            self.brake = None;
            self.motion.set_envelope_rate(None, false);
            return;
        }
        let t = b.elapsed_frames as f32 / b.total_frames as f32;
        self.brake = Some(b);
        self.motion
            .set_envelope_rate(Some(b.start_rate * (1.0 - t)), false);
    }

    fn effective_speed(&self) -> f32 {
        self.playback_ratio() * self.motion.rate()
    }

    /// True while jog may advance the playhead without transport Playing
    /// (vinyl scratch, release ramp, pitch bend, envelope).
    pub fn jog_driving_audio(&self) -> bool {
        self.brake.is_some() || self.motion.driving()
    }

    /// Seek to a specific position (in source frames at the file sample rate).
    pub fn seek(&mut self, frames: i64) -> Result<()> {
        self.cancel_brake();
        self.position_frames = frames;
        self.position_frac = frames as f64;
        self.shadow_position_frac = None;
        self.reset_resampler_state();
        self.reset_stretcher_state();
        self.stretch_active = false;
        Ok(())
    }

    /// Seek audible + shadow (manual seek / catch-up).
    pub fn seek_ms(&mut self, ms: i32) -> Result<()> {
        self.seek_ms_inner(ms, true)
    }

    /// Seek audible only; shadow keeps advancing when slip is on.
    pub fn seek_ms_audible_only(&mut self, ms: i32) -> Result<()> {
        self.seek_ms_inner(ms, false)
    }

    fn seek_ms_inner(&mut self, ms: i32, move_shadow: bool) -> Result<()> {
        self.cancel_brake();
        let audio = self
            .loaded
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No track loaded"))?;
        let frames = f64::from(ms) * f64::from(audio.sample_rate) / 1000.0;
        self.position_frac = frames;
        self.position_frames = frames.floor() as i64;
        if move_shadow {
            // Permanent seek: the old cursor is stale, but an action still in
            // flight keeps one at the new position so its own exit catches up.
            self.shadow_position_frac = None;
            self.rearm_slip_shadow();
        }
        self.reset_resampler_state();
        self.reset_stretcher_state();
        self.stretch_active = false;
        Ok(())
    }

    pub fn slip_enabled(&self) -> bool {
        self.slip_enabled
    }

    pub fn set_slip_enabled(&mut self, enabled: bool) -> Result<()> {
        if self.slip_enabled == enabled {
            return Ok(());
        }
        self.slip_enabled = enabled;
        if enabled {
            // An action already in flight (loop / cue hold / live vinyl scratch)
            // must arm its cursor now, or its exit would skip the catch-up. With
            // nothing in flight this stays `None`, which is the whole point.
            self.rearm_slip_shadow();
        } else {
            self.shadow_position_frac = None;
        }
        Ok(())
    }

    pub fn shadow_position_ms(&self) -> Option<i32> {
        let audio = self.loaded.as_ref()?;
        self.shadow_position_frac
            .map(|frac| secs_to_ms(frac / f64::from(audio.sample_rate)))
    }

    /// Arm the slip cursor at the current audible position (idempotent).
    fn capture_slip_shadow(&mut self) {
        if self.slip_enabled && self.shadow_position_frac.is_none() {
            self.shadow_position_frac = Some(self.position_frac);
        }
    }

    /// Re-arm at the current position when a *different* manual action is still
    /// in flight. A release clears only the cursor it caused to advance: with a
    /// loop, cue hold, or live jog still holding the cursor, dropping it here
    /// would make that action's own exit catch-up silently no-op.
    ///
    /// Only ever arms while the deck can advance a cursor — `Playing` or an
    /// active brake, exactly the states `advance_slip_shadow` renders. A fully
    /// paused deck would freeze the cursor while a scratch keeps moving the
    /// playhead, so the next catch-up would snap the scratch back to this stale
    /// anchor.
    fn rearm_slip_shadow(&mut self) {
        if self.state != DeckState::Playing && self.brake.is_none() {
            return;
        }
        let playing = self.state == DeckState::Playing;
        // A held jog owns a cursor only once it is a live vinyl scratch, exactly
        // the gate `set_jog_touch` applies before arming. A paused or non-vinyl
        // touch has no motor-speed cursor to keep: arming one there would freeze
        // while the playhead moves and snap the scratch back on release.
        let live_vinyl_touch = self.motion.touching()
            && playing
            && self.motion.active_mode(playing) == MotionMode::Vinyl;
        if self.slip_enabled
            && self.shadow_position_frac.is_none()
            && (self.loop_region.is_some() || self.cue_hold_return.is_some() || live_vinyl_touch)
        {
            self.shadow_position_frac = Some(self.position_frac);
        }
    }

    fn catch_up_slip(&mut self) {
        if !self.slip_enabled {
            return;
        }
        let Some(shadow) = self.shadow_position_frac else {
            return;
        };
        self.position_frac = shadow;
        self.position_frames = shadow.floor() as i64;
        self.reset_resampler_state();
        self.reset_stretcher_state();
        self.stretch_active = false;
    }

    /// Advance the slip shadow for one output buffer.
    ///
    /// The shadow advances only while a cursor is armed by an in-flight manual
    /// action; with `None` nothing is calculated.
    ///
    /// Where the shadow does advance it tracks the audible playhead, which runs
    /// at motor speed: normal playback (`Playing`) and a vinyl brake spin-down
    /// (paused but still rendering the ramping motor rate). While the user
    /// scratches, the shadow holds normal speed by design, so those paused
    /// jog-driven states deliberately do not advance it.
    fn advance_slip_shadow(&mut self, frames: usize) {
        if !self.slip_enabled {
            return;
        }
        let Some(shadow) = self.shadow_position_frac else {
            return;
        };
        // Brake renders via `play_interpolated` at `effective_speed`; mirror it
        // exactly so a spin-down cannot leave the shadow behind the playhead.
        // The rate needs no lower clamp: Pad FX envelopes and jog release ramps
        // can make it negative, and the shadow must then move backward with the
        // audible position instead of stalling at zero.
        let rate = if self.brake.is_some() {
            f64::from(self.effective_speed())
        } else if self.state == DeckState::Playing {
            f64::from(self.playback_ratio().max(0.01))
        } else {
            return;
        };
        let Some(audio) = self.loaded.as_ref() else {
            return;
        };
        let src_step = rate * f64::from(audio.sample_rate) / f64::from(self.sample_rate);
        let mut next = shadow + src_step * frames as f64;
        let total = audio.samples.len() / 2;
        if total > 0 {
            next = next.min(total as f64);
        }
        self.shadow_position_frac = Some(next);
    }

    /// Clear loaded audio and reset transport state.
    pub fn unload(&mut self) -> Result<()> {
        self.cancel_brake();
        self.state = DeckState::Stopped;
        self.position_frames = 0;
        self.position_frac = 0.0;
        self.loaded = None;
        self.stems = None;
        self.stem_mute = [false; 4];
        self.stem_isolate = None;
        self.resampler = None;
        self.loop_region = None;
        self.slip_enabled = false;
        self.shadow_position_frac = None;
        self.cue_point_ms = None;
        self.cue_hold_return = None;
        self.track_bpm = None;
        self.ratio_override = None;
        self.buffer.clear();
        self.stretcher = None;
        self.stretch_active = false;
        self.key_shift_semitones = 0.0;
        Ok(())
    }

    /// Store the temporary cue point at the given source time in milliseconds.
    /// Negative values are stored as-is (no ≥0 clamp).
    pub fn set_cue_point_ms(&mut self, ms: i32) -> Result<()> {
        self.cue_point_ms = Some(ms);
        Ok(())
    }

    /// Jump to cue and play while the button is held.
    pub fn begin_cue_hold(&mut self) -> Result<()> {
        let cue_ms = self
            .cue_point_ms
            .or_else(|| self.position_ms())
            .ok_or_else(|| anyhow::anyhow!("No track loaded"))?;
        let return_pos = self.position_ms().unwrap_or(0);
        let was_playing = self.state == DeckState::Playing;
        self.cue_hold_return = Some((return_pos, was_playing));
        self.capture_slip_shadow();
        self.seek_ms_audible_only(cue_ms)?;
        self.play()?;
        Ok(())
    }

    /// Return to the pre-hold transport state.
    pub fn end_cue_hold(&mut self) -> Result<()> {
        let Some((return_pos, was_playing)) = self.cue_hold_return.take() else {
            return Ok(());
        };
        if self.slip_enabled && self.shadow_position_frac.is_some() {
            self.catch_up_slip();
            self.shadow_position_frac = None;
        } else {
            self.seek_ms(return_pos)?;
        }
        self.rearm_slip_shadow();
        if was_playing {
            self.play()?;
        } else {
            self.pause()?;
        }
        Ok(())
    }

    /// Activate a loop region from source-time milliseconds.
    /// Stores inclusive/exclusive bounds as source frames (not seconds).
    pub fn set_loop_region_ms(&mut self, in_ms: i32, out_ms: i32) -> Result<()> {
        let rate = f64::from(
            self.loaded
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("No track loaded"))?
                .sample_rate,
        );
        if out_ms <= in_ms {
            return Err(anyhow::anyhow!("Loop out must be after loop in"));
        }
        self.capture_slip_shadow();
        let in_frames = f64::from(in_ms) * rate / 1000.0;
        let out_frames = f64::from(out_ms) * rate / 1000.0;
        self.loop_region = Some((in_frames, out_frames));
        Ok(())
    }

    /// Clear the active loop region.
    pub fn clear_loop(&mut self) {
        if self.slip_enabled && self.loop_region.is_some() {
            self.catch_up_slip();
            self.shadow_position_frac = None;
        }
        self.loop_region = None;
        // After the region drops, only a concurrent cue hold or live jog still
        // needs the cursor; re-arm it there so its own exit still catches up.
        self.rearm_slip_shadow();
    }

    /// Clear resampler filter/history state after a position jump (seek/stop).
    fn reset_resampler_state(&mut self) {
        if let Some(resampler) = self.resampler.as_mut() {
            resampler.reset();
        }
    }

    /// Build the stretcher on demand if none is installed.
    ///
    /// Heavy. Callers that hold the DSP lock should instead take a
    /// [`StretcherSpec`](Self::stretcher_spec), build it off-lock, and
    /// [`install`](Self::install_stretcher) it. The remaining under-lock
    /// callers are `set_key_lock` and `set_key_shift_semitones`; moving those
    /// off-lock is a follow-up, since the engine has to pre-build before it
    /// takes the lock. Track load builds off-lock via [`DeckLoadSpec::prepare`].
    fn ensure_stretcher(&mut self) -> Result<()> {
        if let Some(spec) = self.stretcher_spec() {
            self.stretcher = Some(spec.build()?.0);
        }
        Ok(())
    }

    /// Build recipe for the stretcher, or `None` when one is already installed.
    /// Construction is heavy; callers holding the DSP lock should build via
    /// [`StretcherSpec::build`] first and install with [`Self::install_stretcher`].
    pub fn stretcher_spec(&self) -> Option<StretcherSpec> {
        if self.stretcher.is_some() {
            return None;
        }
        Some(StretcherSpec {
            sample_rate: self.sample_rate,
            buffer_size: self.buffer_size,
        })
    }

    /// Install a stretcher built off-lock from [`Self::stretcher_spec`].
    /// A prepared stretcher is dropped (with a warning) if one is already installed.
    pub fn install_stretcher(&mut self, prepared: PreparedStretcher) {
        if self.stretcher.is_none() {
            self.stretcher = Some(prepared.0);
        } else {
            tracing::warn!(
                "Deck {}: stretcher already installed; dropping prepared stretcher",
                self.id
            );
        }
    }

    fn reset_stretcher_state(&mut self) {
        if let Some(stretcher) = self.stretcher.as_mut() {
            stretcher.reset();
        }
    }

    /// Snapshot the parameters needed to build load resources off the DSP lock.
    pub fn load_spec(&self) -> DeckLoadSpec {
        DeckLoadSpec {
            deck_id: self.id,
            sample_rate: self.sample_rate,
            buffer_size: self.buffer_size,
            resampler_quality: self.resampler_quality.clone(),
            key_lock: self.key_lock,
        }
    }

    /// Swap in a load built by [`DeckLoadSpec::prepare`]. Infallible: only resets playback
    /// state and assigns the prepared resources.
    pub fn apply_prepared_load(&mut self, prepared: PreparedDeckLoad) {
        // A brake started in pause must not ride onto the new track: it would
        // render the freshly loaded audio at the decaying rate while paused.
        self.cancel_brake();
        let PreparedDeckLoad {
            audio,
            resampler,
            stretcher,
        } = prepared;
        self.position_frames = 0;
        self.position_frac = 0.0;
        self.loop_region = None;
        self.slip_enabled = false;
        self.shadow_position_frac = None;
        self.cue_point_ms = Some(0);
        self.cue_hold_return = None;
        self.loaded = Some(audio);
        self.stems = None;
        self.stem_mute = [false; 4];
        self.stem_isolate = None;
        // Session-only key shift must not leak onto a newly loaded track.
        self.key_shift_semitones = 0.0;
        self.resampler = resampler;
        if let Some(stretcher) = stretcher {
            self.stretcher = Some(stretcher);
        }
        self.reset_stretcher_state();
        self.stretch_active = false;

        if let Some(loaded) = self.loaded.as_ref() {
            tracing::info!(
                "Loaded audio into deck {} from {}: {} source frames at {} Hz ({} channels, engine: {} Hz)",
                self.id,
                loaded.source_id,
                loaded.samples.len() / 2,
                loaded.sample_rate,
                loaded.channels,
                self.sample_rate
            );
        }
    }

    /// Load shared decoded audio. Creates a resampler when the source rate differs from the engine rate.
    pub fn load(&mut self, audio: Arc<LoadedAudio>) -> Result<()> {
        let prepared = self.load_spec().prepare(audio)?;
        self.apply_prepared_load(prepared);
        Ok(())
    }

    /// Attach four stem buffers (drums, bass, other, vocals — NI index order). Cleared on next [`Self::load`].
    pub fn attach_stems(&mut self, stems: [Arc<LoadedAudio>; 4]) {
        self.stems = Some(stems);
        self.stem_mute = [false; 4];
        self.stem_isolate = None;
        self.stem_gain_current = [1.0; 4];
    }

    /// Clear attached stems (keeps the main buffer).
    pub fn clear_stems(&mut self) {
        self.stems = None;
        self.stem_mute = [false; 4];
        self.stem_isolate = None;
        self.stem_gain_current = [1.0; 4];
    }

    pub fn stems_ready(&self) -> bool {
        self.stems.is_some()
    }

    pub fn stem_mute(&self) -> [bool; 4] {
        self.stem_mute
    }

    pub fn stem_isolate(&self) -> Option<u8> {
        self.stem_isolate
    }

    pub fn set_stem_mute(&mut self, index: usize, muted: bool) {
        if index < 4 {
            self.stem_mute[index] = muted;
        }
    }

    pub fn toggle_stem_mute(&mut self, index: usize) {
        if index < 4 {
            self.stem_mute[index] = !self.stem_mute[index];
        }
    }

    /// Isolate one stem, or clear isolate when the same index is pressed again.
    pub fn toggle_stem_isolate(&mut self, index: u8) {
        if usize::from(index) >= 4 {
            return;
        }
        self.stem_isolate = match self.stem_isolate {
            Some(current) if current == index => None,
            _ => Some(index),
        };
    }

    /// Steady-state gain for a stem from mute/isolate (before smoothing).
    fn stem_target_gain(&self, index: usize) -> f32 {
        if let Some(iso) = self.stem_isolate {
            return if usize::from(iso) == index { 1.0 } else { 0.0 };
        }
        if self.stem_mute.get(index).copied().unwrap_or(false) {
            0.0
        } else {
            1.0
        }
    }

    /// Advance smoothed stem gains one output sample toward their targets (~5 ms).
    fn ramp_stem_gains_one_sample(&mut self) {
        const RAMP_SECS: f64 = 0.005;
        let step = (1.0 / (RAMP_SECS * f64::from(self.sample_rate.max(1)))) as f32;
        for i in 0..4 {
            let target = self.stem_target_gain(i);
            let cur = self.stem_gain_current[i];
            if (cur - target).abs() <= step {
                self.stem_gain_current[i] = target;
            } else if cur < target {
                self.stem_gain_current[i] = cur + step;
            } else {
                self.stem_gain_current[i] = cur - step;
            }
        }
    }

    /// Mix stems (or main buffer) at a playhead expressed in **main-track** frames.
    fn mix_at_position(
        &self,
        pos: f64,
        fallback: &[Sample],
        main_sample_rate: u32,
    ) -> (Sample, Sample) {
        let Some(stems) = self.stems.as_ref() else {
            return interpolate_stereo(fallback, pos);
        };
        let mut left = 0.0;
        let mut right = 0.0;
        for (i, stem) in stems.iter().enumerate() {
            let g = self.stem_gain_current[i];
            if g == 0.0 {
                continue;
            }
            // Stem WAVs may be 44.1 kHz while the deck main buffer is engine-rate.
            let stem_pos = if main_sample_rate == 0 || stem.sample_rate == main_sample_rate {
                pos
            } else {
                pos * f64::from(stem.sample_rate) / f64::from(main_sample_rate)
            };
            let (sl, sr) = interpolate_stereo(&stem.samples, stem_pos);
            left += sl * g;
            right += sr * g;
        }
        (left, right)
    }

    /// Borrow the loaded audio reference, if any.
    pub fn loaded_audio(&self) -> Option<&Arc<LoadedAudio>> {
        self.loaded.as_ref()
    }

    /// Check if audio is loaded
    pub fn has_audio_loaded(&self) -> bool {
        self.loaded.is_some()
    }

    /// Get the loaded file path
    pub fn file_path(&self) -> Option<&str> {
        self.loaded.as_ref().map(|audio| audio.source_id.as_str())
    }

    /// Get the duration of loaded audio in seconds
    pub fn duration_seconds(&self) -> Option<f64> {
        let audio = self.loaded.as_ref()?;
        let frame_count = audio.samples.len() / 2;
        Some(frame_count as f64 / audio.sample_rate as f64)
    }

    /// Process audio for this deck.
    ///
    /// # Arguments
    /// * `frames` - Number of frames to process (should match the engine buffer size)
    pub fn process(&mut self, frames: usize) -> Result<&[Sample]> {
        self.advance_brake(frames);
        self.tick_jog(frames);

        let transport_playing = self.state == DeckState::Playing;
        if !transport_playing && !self.jog_driving_audio() {
            // Return silence if not playing. resize() only zero-fills new capacity;
            // after playback the buffer may already be the right length with stale audio.
            self.buffer.resize(frames * 2, 0.0);
            self.buffer.fill(0.0);
            return Ok(&self.buffer);
        }

        // Ensure buffer is large enough for stereo output
        let buffer_size = frames * 2;
        self.buffer.resize(buffer_size, 0.0);

        // Play loaded audio samples if available, otherwise generate test audio
        if let Some(loaded) = self.loaded.clone() {
            let source_rate = loaded.sample_rate;
            // Stems bypass key-lock stretch; keep stretch_active in sync with the path we take.
            // Key shift forces the stretch path even with stems so stems still pitch-shift.
            // Key shift and key lock both yield to the vinyl/interpolated path
            // while the jog drives audio, so a latched shift never blocks
            // scratching.
            let want_stretch = !self.jog_driving_audio()
                && (self.key_shift_semitones != 0.0 || (self.key_lock && self.stems.is_none()));
            if self.stretch_active && !want_stretch {
                self.reset_stretcher_state();
            }
            if !want_stretch {
                // Leaving the stretch path re-arms the missing-stretcher note so
                // the next stretch episode logs its own failure once.
                self.stretcher_fallback_logged = false;
            }
            self.stretch_active = want_stretch;

            if want_stretch {
                if self.stretcher.is_some() {
                    self.play_stretched(frames, &loaded.samples, source_rate);
                } else {
                    if !self.stretcher_fallback_logged {
                        tracing::error!(
                            "Deck {}: stretcher missing; falling back to vinyl",
                            self.id
                        );
                        self.stretcher_fallback_logged = true;
                    }
                    self.stretch_active = false;
                    self.play_interpolated(frames, &loaded.samples, source_rate);
                }
            } else {
                // shortcut: stem mix uses interpolated path (key-lock stretch skipped while stems attached).
                let eff = self.effective_speed();
                let use_interp = self.stems.is_some()
                    || (eff - 1.0).abs() > f32::EPSILON
                    || self.loop_region.is_some()
                    || self.position_frac < 0.0
                    || !transport_playing;
                if use_interp || self.resampler.is_none() {
                    self.play_interpolated(frames, &loaded.samples, source_rate);
                } else {
                    let source_frames = self.play_loaded_audio(frames, &loaded.samples);
                    self.position_frames += source_frames as i64;
                    self.position_frac = self.position_frames as f64;
                }
            }
        } else {
            static NO_TRACK_WARN: AtomicU32 = AtomicU32::new(0);
            if NO_TRACK_WARN.fetch_add(1, Ordering::Relaxed) == 0 {
                tracing::warn!(
                    "Deck {} is playing but no track is loaded; outputting silence",
                    self.id
                );
            }
            self.buffer.fill(0.0);
        }

        self.advance_slip_shadow(frames);

        if transport_playing {
            self.check_track_end();
        }

        Ok(&self.buffer)
    }

    /// Render loaded audio with fractional stepping (vinyl tempo + loops).
    fn play_interpolated(&mut self, frames: usize, audio_samples: &[Sample], source_rate: u32) {
        let buffer_size = frames * 2;
        self.buffer.resize(buffer_size, 0.0);

        let total_source_frames = audio_samples.len() / 2;
        let step =
            self.effective_speed() as f64 * f64::from(source_rate) / f64::from(self.sample_rate);

        for out in 0..frames {
            if self.position_frac < 0.0 {
                self.buffer[out * 2] = 0.0;
                self.buffer[out * 2 + 1] = 0.0;
                if self.stems.is_some() {
                    self.ramp_stem_gains_one_sample();
                }
                self.position_frac += step;
                continue;
            }

            if self.position_frac >= total_source_frames as f64 {
                self.buffer[out * 2] = 0.0;
                self.buffer[out * 2 + 1] = 0.0;
                if self.stems.is_some() {
                    self.ramp_stem_gains_one_sample();
                }
                continue;
            }

            if self.stems.is_some() {
                self.ramp_stem_gains_one_sample();
            }
            let (left, right) =
                self.mix_at_position(self.position_frac, audio_samples, source_rate);
            self.buffer[out * 2] = left;
            self.buffer[out * 2 + 1] = right;
            self.position_frac += step;

            if let Some((loop_in, loop_out)) = self.loop_region {
                if self.position_frac >= loop_out {
                    let span = loop_out - loop_in;
                    if span > 0.0 {
                        self.position_frac = loop_in + ((self.position_frac - loop_out) % span);
                    }
                }
            }
        }

        self.position_frames = self.position_frac.floor() as i64;
    }

    /// Read one interleaved stereo frame for the stretch feed: stem mix when
    /// stems are attached (so key shift keeps stems audible), else the main buffer.
    fn read_frame_at(
        &self,
        pos: f64,
        fallback: &[Sample],
        main_sample_rate: u32,
    ) -> (Sample, Sample) {
        if self.stems.is_some() {
            self.mix_at_position(pos, fallback, main_sample_rate)
        } else {
            interpolate_stereo(fallback, pos)
        }
    }

    /// Key-lock path: time-stretch at engine rate with pitch held, plus key shift.
    ///
    /// timestretch tempo_rate is playback speed (`>1` = faster).
    fn play_stretched(&mut self, frames: usize, audio_samples: &[Sample], source_rate: u32) {
        let buffer_size = frames * 2;
        self.buffer.resize(buffer_size, 0.0);

        let playback_ratio = f64::from(self.playback_ratio().max(0.01));
        let src_step = f64::from(source_rate) / f64::from(self.sample_rate);
        let pitch_factor = self.pitch_factor();

        // Move the stretcher and output buffer out of `self` so the feed closure can
        // borrow `self` for stem mixing (gains + read) while key shift is engaged.
        let mut stretcher_opt = self.stretcher.take();
        let Some(stretcher) = stretcher_opt.as_mut() else {
            self.buffer.fill(0.0);
            return;
        };
        stretcher.set_tempo_rate(playback_ratio);
        stretcher.set_pitch_factor(pitch_factor);

        let loop_region = self.loop_region;
        let mut position_frac = self.position_frac;

        let mut out = std::mem::take(&mut self.buffer);
        out.resize(buffer_size, 0.0);

        let stats = stretcher.pull_interleaved(frames, &mut out, &mut |need, buf| {
            let mut produced = 0usize;
            for i in 0..need {
                // Smooth stem mute/isolate gains on the stretch path too (the
                // interpolated path ramps per output frame); `mix_at_position`
                // reads these gains when stems are attached.
                if self.stems.is_some() {
                    self.ramp_stem_gains_one_sample();
                }
                if position_frac < 0.0 {
                    buf[i * 2] = 0.0;
                    buf[i * 2 + 1] = 0.0;
                    position_frac += src_step;
                    produced += 1;
                    continue;
                }

                let total_source_frames = audio_samples.len() / 2;
                if position_frac >= total_source_frames as f64 {
                    buf[i * 2] = 0.0;
                    buf[i * 2 + 1] = 0.0;
                    produced += 1;
                    continue;
                }

                let (left, right) = self.read_frame_at(position_frac, audio_samples, source_rate);
                buf[i * 2] = left;
                buf[i * 2 + 1] = right;
                position_frac += src_step;
                produced += 1;

                if let Some((loop_in, loop_out)) = loop_region {
                    if position_frac >= loop_out {
                        let span = loop_out - loop_in;
                        if span > 0.0 {
                            position_frac = loop_in + ((position_frac - loop_out) % span);
                        }
                    }
                }
            }
            produced
        });

        self.buffer = out;
        self.stretcher = stretcher_opt;
        self.position_frac = position_frac;
        self.position_frames = self.position_frac.floor() as i64;
        let _ = stats;
    }

    /// Render loaded audio into `self.buffer`.
    ///
    /// Returns the number of source frames consumed (at the file sample rate).
    fn play_loaded_audio(&mut self, frames: usize, audio_samples: &[Sample]) -> u64 {
        if self.position_frac < 0.0 {
            self.buffer.fill(0.0);
            return 0;
        }

        let start_pos = self.position_frac as usize * 2;

        if self.position_frames.rem_euclid(1000) == 0 {
            tracing::debug!(
                "Deck {}: position={}, start_pos={}, audio_len={}, frames={}",
                self.id,
                self.position_frames,
                start_pos,
                audio_samples.len(),
                frames
            );
        }

        if start_pos >= audio_samples.len() {
            self.buffer.fill(0.0);
            return 0;
        }

        if self.resampler.is_none() {
            return self.copy_source_passthrough(frames, audio_samples, start_pos);
        }

        if let Some(resampler) = self.resampler.as_mut() {
            resampler.set_output_chunk_frames(frames);
        }
        self.resample_into_buffer(frames, audio_samples, start_pos)
    }

    fn copy_source_passthrough(
        &mut self,
        frames: usize,
        audio_samples: &[Sample],
        start_pos: usize,
    ) -> u64 {
        let available_samples = audio_samples.len() - start_pos;
        let samples_to_copy = std::cmp::min(frames * 2, available_samples);

        self.buffer[..samples_to_copy]
            .copy_from_slice(&audio_samples[start_pos..start_pos + samples_to_copy]);
        self.buffer[samples_to_copy..].fill(0.0);

        (samples_to_copy / 2) as u64
    }

    fn resample_into_buffer(
        &mut self,
        frames: usize,
        audio_samples: &[Sample],
        mut src_pos: usize,
    ) -> u64 {
        let output_frames = frames;
        let mut out_frames = 0usize;
        let mut total_input_frames = 0usize;
        let resampler = match self.resampler.as_mut() {
            Some(r) => r,
            None => return 0,
        };

        while out_frames < output_frames && src_pos < audio_samples.len() {
            let need_in = resampler.input_frames_next();
            let step_out = resampler
                .output_frames_next()
                .min(output_frames - out_frames);

            let remaining_src = (audio_samples.len() - src_pos) / 2;
            if remaining_src == 0 {
                break;
            }

            let in_frames = need_in.min(remaining_src);
            let chunk = &audio_samples[src_pos..src_pos + in_frames * 2];
            let out_start = out_frames * 2;
            let out_len = step_out * 2;

            let (out_samples, consumed) =
                resampler.process(chunk, &mut self.buffer[out_start..out_start + out_len], 2);

            let produced = out_samples / 2;
            if produced == 0 && consumed == 0 {
                break;
            }

            out_frames += produced;
            total_input_frames += consumed;
            src_pos += consumed * 2;
        }

        static mut RESAMPLE_LOG: u32 = 0;
        unsafe {
            RESAMPLE_LOG += 1;
            if RESAMPLE_LOG % 500 == 1 {
                tracing::info!(
                    "Deck {} resample: pos={}, consumed={}, out_frames={}/{}",
                    self.id,
                    self.position_frames,
                    total_input_frames,
                    out_frames,
                    output_frames
                );
            }
        }

        if out_frames * 2 < self.buffer.len() {
            self.buffer[out_frames * 2..].fill(0.0);
        }

        total_input_frames as u64
    }
}

fn interpolate_stereo(samples: &[Sample], pos: f64) -> (Sample, Sample) {
    let frame = pos.floor() as usize;
    let frac = (pos - frame as f64) as Sample;
    let i0 = frame * 2;
    if i0 + 3 >= samples.len() {
        if i0 + 1 < samples.len() {
            return (samples[i0], samples[i0 + 1]);
        }
        return (0.0, 0.0);
    }
    let l0 = samples[i0];
    let r0 = samples[i0 + 1];
    let l1 = samples[i0 + 2];
    let r1 = samples[i0 + 3];
    (l0 + (l1 - l0) * frac, r0 + (r1 - r0) * frac)
}

/// Immutable deck parameters needed to build load resources off the DSP lock.
#[derive(Debug, Clone)]
pub struct DeckLoadSpec {
    deck_id: usize,
    sample_rate: u32,
    buffer_size: u32,
    resampler_quality: String,
    key_lock: bool,
}

/// Resampler and stretcher built for a track, ready to swap in under the DSP lock.
pub struct PreparedDeckLoad {
    audio: Arc<LoadedAudio>,
    resampler: Option<Box<dyn Resampler>>,
    stretcher: Option<Box<dyn TimeStretcher>>,
}

/// Build a time stretcher at the engine rate, sized to the device callback (at least one chunk).
/// Single construction site for [`DeckLoadSpec::prepare`], [`StretcherSpec::build`],
/// and [`Deck::ensure_stretcher`].
fn new_stretcher(sample_rate: u32, buffer_size: u32) -> Result<Box<dyn TimeStretcher>> {
    create_stretcher(sample_rate, buffer_size.max(Buffer::LEN as u32) as usize)
}

impl DeckLoadSpec {
    /// Build the resampler and, when key lock is on, the stretcher for `audio`.
    ///
    /// All fallible construction happens here without touching deck state, so callers
    /// can run it without holding the DSP lock.
    pub fn prepare(&self, audio: Arc<LoadedAudio>) -> Result<PreparedDeckLoad> {
        let source_rate = audio.sample_rate;
        let resampler = if source_rate == self.sample_rate {
            tracing::info!(
                "Deck {} passthrough at {} Hz (no resampling)",
                self.deck_id,
                self.sample_rate
            );
            None
        } else {
            // Mixer processes dasp_graph Buffer::LEN chunks, not the device buffer size.
            // Rubato FixedSync::Output must match that quantum or it returns (0,0) forever.
            let built = resampler::create_resampler(
                source_rate,
                self.sample_rate,
                2,
                Buffer::LEN,
                Some(&self.resampler_quality),
            )?;
            tracing::info!(
                "Deck {} realtime resampler: {} Hz -> {} Hz (chunk_frames={}, quality={})",
                self.deck_id,
                source_rate,
                self.sample_rate,
                Buffer::LEN,
                self.resampler_quality
            );
            Some(built)
        };
        let stretcher = if self.key_lock {
            Some(new_stretcher(self.sample_rate, self.buffer_size)?)
        } else {
            None
        };
        Ok(PreparedDeckLoad {
            audio,
            resampler,
            stretcher,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use audio_core::LoadedAudio;
    use std::sync::Arc;

    const ENGINE_RATE: u32 = 48000;
    const CHUNK: usize = 512;

    fn new_deck(chunk_frames: usize) -> Deck {
        Deck::new(0, ENGINE_RATE, chunk_frames as u32, "medium")
    }

    fn load_test_samples(deck: &mut Deck, samples: Vec<Sample>, sample_rate: u32) {
        let audio = LoadedAudio {
            samples,
            sample_rate,
            channels: 2,
            source_id: "test.wav".to_string(),
        };
        deck.load(Arc::new(audio)).unwrap();
    }

    /// Estimate dominant frequency (Hz) via zero-crossings over the last quarter.
    fn estimate_frequency(samples: &[f32], sample_rate: u32) -> f64 {
        let frame_count = samples.len() / 2;
        let start = frame_count * 3 / 4;
        let mono: Vec<f32> = (start..frame_count).map(|i| samples[i * 2]).collect();
        if mono.len() < 2 {
            return 0.0;
        }
        let crossings = mono
            .windows(2)
            .filter(|w| (w[0] <= 0.0 && w[1] > 0.0) || (w[0] >= 0.0 && w[1] < 0.0))
            .count();
        let secs = mono.len() as f64 / f64::from(sample_rate);
        crossings as f64 / (2.0 * secs)
    }

    /// Max per-chunk RMS across `chunks` process calls (covers stretch warm-up).
    fn max_rms_over(deck: &mut Deck, chunks: usize) -> f32 {
        let mut rms = 0.0f32;
        for _ in 0..chunks {
            let out = deck.process(CHUNK).unwrap();
            rms = rms.max((out.iter().map(|s| s * s).sum::<f32>() / out.len() as f32).sqrt());
        }
        rms
    }

    #[test]
    fn key_shift_changes_pitch_not_tempo() {
        let mut deck = new_deck(CHUNK);
        // 440 Hz sine at engine rate, 2 s.
        let n = ENGINE_RATE as usize * 2;
        let mut samples = vec![0.0f32; n * 2];
        for i in 0..n {
            let v =
                ((i as f64) * 440.0 / f64::from(ENGINE_RATE) * std::f64::consts::TAU).sin() as f32;
            samples[i * 2] = v;
            samples[i * 2 + 1] = v;
        }
        load_test_samples(&mut deck, samples, ENGINE_RATE);
        deck.set_key_shift_semitones(12.0).unwrap();
        deck.play().unwrap();
        let before = deck.position_frames();
        let mut out = Vec::new();
        for _ in 0..64 {
            out.extend_from_slice(deck.process(CHUNK).unwrap());
        }
        // Pitch doubled.
        let f = estimate_frequency(&out, ENGINE_RATE);
        assert!((f - 880.0).abs() / 880.0 < 0.05, "got {f}");
        // Tempo unchanged: ~64 * CHUNK source frames advanced.
        let advanced = deck.position_frames() - before;
        assert!((advanced - (64 * CHUNK) as i64).abs() < 2_000);
    }

    #[test]
    fn key_shift_zero_matches_unset_output() {
        let mut a = new_deck(CHUNK);
        let mut b = new_deck(CHUNK);
        let n = ENGINE_RATE as usize;
        let samples: Vec<f32> = (0..n * 2).map(|i| (i as f32 * 0.01).sin()).collect();
        load_test_samples(&mut a, samples.clone(), ENGINE_RATE);
        load_test_samples(&mut b, samples, ENGINE_RATE);
        a.set_key_shift_semitones(0.0).unwrap();
        a.play().unwrap();
        b.play().unwrap();
        for _ in 0..8 {
            let x = a.process(CHUNK).unwrap().to_vec();
            let y = b.process(CHUNK).unwrap().to_vec();
            assert_eq!(x, y);
        }
    }

    #[test]
    fn key_shift_clamps_and_rejects_non_finite() {
        let mut deck = new_deck(CHUNK);
        load_test_samples(&mut deck, vec![0.1; CHUNK * 2 * 8_000], ENGINE_RATE);
        deck.set_key_shift_semitones(f32::NAN).unwrap();
        assert_eq!(deck.key_shift_semitones(), 0.0);
        deck.set_key_shift_semitones(120.0).unwrap();
        assert_eq!(deck.key_shift_semitones(), 16.0);
    }

    /// Four identical audible stems attached to a fresh deck, with `key_shift` set.
    fn deck_with_stems(key_shift: f32) -> Deck {
        let mut deck = new_deck(CHUNK);
        load_test_samples(&mut deck, vec![0.0; CHUNK * 2 * 8_000], ENGINE_RATE);
        let stem = Arc::new(LoadedAudio {
            samples: (0..CHUNK * 2 * 8_000)
                .map(|i| (i as f32 * 0.02).sin() * 0.5)
                .collect(),
            sample_rate: ENGINE_RATE,
            channels: 2,
            source_id: "stem.wav".into(),
        });
        deck.attach_stems([
            Arc::clone(&stem),
            Arc::clone(&stem),
            Arc::clone(&stem),
            Arc::clone(&stem),
        ]);
        deck.set_key_shift_semitones(key_shift).unwrap();
        deck
    }

    #[test]
    fn key_shift_with_stems_keeps_stem_audio() {
        let mut deck = deck_with_stems(2.0);
        deck.play().unwrap();
        // The time-stretch engine has a pipeline latency of ~2 chunks, so let it
        // prime before measuring that stem audio is present.
        let rms = max_rms_over(&mut deck, 8);
        assert!(
            rms > 1e-3,
            "stems must stay audible under key shift, rms={rms}"
        );
    }

    #[test]
    fn key_shift_stem_isolate_is_audible() {
        let mut full = deck_with_stems(2.0);
        full.play().unwrap();
        let full_rms = max_rms_over(&mut full, 16);

        let mut iso = deck_with_stems(2.0);
        iso.toggle_stem_isolate(0);
        iso.play().unwrap();
        let iso_rms = max_rms_over(&mut iso, 16);

        assert!(
            iso_rms < full_rms * 0.5,
            "isolate must lower stem level under key shift: full={full_rms} iso={iso_rms}"
        );
    }

    #[test]
    fn key_shift_resets_on_unload() {
        let mut deck = new_deck(CHUNK);
        load_test_samples(&mut deck, vec![0.1; CHUNK * 2 * 8_000], ENGINE_RATE);
        deck.set_key_shift_semitones(5.0).unwrap();
        deck.unload().unwrap();
        assert_eq!(deck.key_shift_semitones(), 0.0);
    }

    #[test]
    fn key_shift_stretcher_is_prebuilt_off_process() {
        // 440 Hz sine at engine rate, 1 s, so a +5 semitone shift is measurable.
        let n = ENGINE_RATE as usize;
        let sine = || {
            let mut samples = vec![0.0f32; n * 2];
            for i in 0..n {
                let v = ((i as f64) * 440.0 / f64::from(ENGINE_RATE) * std::f64::consts::TAU).sin();
                samples[i * 2] = v as f32;
                samples[i * 2 + 1] = v as f32;
            }
            samples
        };

        let mut deck = new_deck(CHUNK);
        load_test_samples(&mut deck, sine(), ENGINE_RATE);
        let spec = deck.stretcher_spec().expect("no stretcher before install");
        deck.install_stretcher(spec.build().unwrap());
        assert!(deck.stretcher_spec().is_none());

        deck.set_key_shift_semitones(5.0).unwrap();
        deck.play().unwrap();
        let mut out = Vec::new();
        for _ in 0..32 {
            out.extend_from_slice(deck.process(CHUNK).unwrap());
        }
        assert!(
            out.iter().all(|s| s.is_finite()),
            "stretched output must be finite"
        );
        // The stretch path must actually run: a vinyl fallback would leave the
        // pitch at 440 Hz and make this output identical to the control deck.
        let f = estimate_frequency(&out, ENGINE_RATE);
        let expected = 440.0 * 2.0f64.powf(5.0 / 12.0);
        assert!(
            (f - expected).abs() / expected < 0.10,
            "expected ~{expected:.0} Hz for a +5 semitone shift, got {f}"
        );
        let mut vinyl = new_deck(CHUNK);
        load_test_samples(&mut vinyl, sine(), ENGINE_RATE);
        vinyl.play().unwrap();
        let mut vinyl_out = Vec::new();
        for _ in 0..32 {
            vinyl_out.extend_from_slice(vinyl.process(CHUNK).unwrap());
        }
        assert!(
            out.iter()
                .zip(&vinyl_out)
                .any(|(a, b)| (a - b).abs() > 1e-3),
            "stretched output must differ from the vinyl path"
        );
        assert!(
            deck.stretcher_spec().is_none(),
            "process must not build a stretcher"
        );
    }

    #[test]
    fn key_shift_builds_stretcher_on_control_call() {
        let mut deck = new_deck(CHUNK);
        load_test_samples(&mut deck, vec![0.4f32; CHUNK * 2 * 8_000], ENGINE_RATE);
        assert!(deck.stretcher_spec().is_some());

        deck.set_key_shift_semitones(5.0).unwrap();
        assert!(
            deck.stretcher_spec().is_none(),
            "setting a non-zero shift must install the stretcher before the first process"
        );
        deck.play().unwrap();
        deck.process(CHUNK).unwrap();
        assert!(
            deck.stretch_active,
            "first stretched process should use the stretcher"
        );
    }

    #[test]
    fn load_resets_key_shift() {
        let mut deck = new_deck(CHUNK);
        load_test_samples(&mut deck, vec![0.1; CHUNK * 2 * 8_000], ENGINE_RATE);
        deck.set_key_shift_semitones(5.0).unwrap();
        // Loading a second track must clear the session-only shift so it never
        // inherits a stale offset.
        load_test_samples(&mut deck, vec![0.1; CHUNK * 2 * 8_000], ENGINE_RATE);
        assert_eq!(deck.key_shift_semitones(), 0.0);
    }

    #[test]
    fn seek_ms_allows_negative_playhead() {
        let mut deck = new_deck(CHUNK);
        load_test_samples(&mut deck, vec![0.0f32; CHUNK * 2], ENGINE_RATE);
        deck.seek_ms(-500).unwrap();
        assert_eq!(deck.position_ms(), Some(-500));
    }

    #[test]
    fn negative_cue_recall_keeps_negative_position() {
        let mut deck = new_deck(CHUNK);
        load_test_samples(&mut deck, vec![0.0f32; CHUNK * 2], ENGINE_RATE);
        deck.set_cue_point_ms(-250).unwrap();
        assert_eq!(deck.cue_point_ms(), Some(-250));
        deck.seek_ms(deck.cue_point_ms().unwrap()).unwrap();
        assert_eq!(deck.position_ms(), Some(-250));
    }

    #[test]
    fn test_deck_creation() {
        let deck = new_deck(CHUNK);
        assert_eq!(deck.id(), 0);
        assert_eq!(deck.state(), &DeckState::Stopped);
        assert_eq!(deck.position_frames(), 0);
        assert_eq!(deck.speed(), 0.5);
        assert_eq!(deck.tempo_range(), crate::tempo::DEFAULT_TEMPO_RANGE);
        assert!((deck.playback_ratio() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_deck_playback_controls() {
        let mut deck = new_deck(CHUNK);

        // Test play
        deck.play().unwrap();
        assert_eq!(deck.state(), &DeckState::Playing);

        // Test pause
        deck.pause().unwrap();
        assert_eq!(deck.state(), &DeckState::Paused);

        // Test stop
        deck.stop().unwrap();
        assert_eq!(deck.state(), &DeckState::Stopped);
        assert_eq!(deck.position_frames(), 0);
    }

    #[test]
    fn test_deck_speed_control() {
        let mut deck = new_deck(CHUNK);

        deck.set_track_bpm(Some(150.0));
        deck.set_speed(0.0).unwrap();
        assert_eq!(deck.speed(), 0.0);
        assert!((deck.playback_ratio() - 1.06).abs() < 1e-5);
        assert!(deck.set_speed(-0.1).is_err());
        assert!(deck.set_speed(1.1).is_err());
    }

    #[test]
    fn test_deck_seek() {
        let mut deck = new_deck(CHUNK);

        deck.seek(1000).unwrap();
        assert_eq!(deck.position_frames(), 1000);
    }

    #[test]
    fn test_deck_audio_processing() {
        let mut deck = new_deck(CHUNK);

        // Test processing when stopped (should return silence)
        let audio = deck.process(CHUNK).unwrap();
        assert_eq!(audio.len(), 1024); // Stereo
        assert!(audio.iter().all(|&s| s == 0.0));

        // Playing with no track loaded outputs silence
        deck.play().unwrap();
        let audio = deck.process(CHUNK).unwrap();
        assert_eq!(audio.len(), 1024);
        assert!(audio.iter().all(|&s| s == 0.0));

        // Loaded audio should produce non-zero samples
        load_test_samples(&mut deck, vec![0.5f32; CHUNK * 2], ENGINE_RATE);
        let audio = deck.process(CHUNK).unwrap();
        assert!(audio.iter().any(|&s| s != 0.0));
    }

    #[test]
    fn process_returns_dry_playback_samples() {
        let mut deck = new_deck(2);
        let samples = vec![0.25, -0.5, 0.75, -1.0];
        load_test_samples(&mut deck, samples.clone(), ENGINE_RATE);
        deck.play().unwrap();

        let output = deck.process(2).unwrap();

        assert_eq!(output, samples);
    }

    #[test]
    fn test_deck_realtime_resample_playback_rate() {
        let input_rate = 44100u32;
        let duration_secs = 10usize;
        let mut deck = new_deck(CHUNK);
        let frames = input_rate as usize * duration_secs;
        let mut samples = vec![0.0f32; frames * 2];
        for i in 0..frames {
            let s = (2.0 * std::f32::consts::PI * 440.0 * i as f32 / input_rate as f32).sin() * 0.5;
            samples[i * 2] = s;
            samples[i * 2 + 1] = s;
        }

        load_test_samples(&mut deck, samples, input_rate);
        deck.play().unwrap();

        // Only full chunks — resampler is sized to the immutable buffer size.
        let total_output_frames = (ENGINE_RATE as usize * duration_secs / CHUNK) * CHUNK;
        let mut produced = 0usize;
        while produced < total_output_frames {
            deck.process(CHUNK).unwrap();
            produced += CHUNK;
        }

        let expected_source =
            (total_output_frames as f64 * input_rate as f64 / ENGINE_RATE as f64) as u64;
        let actual = deck.position_frames();
        let ratio = actual as f64 / expected_source as f64;
        eprintln!("expected_source={expected_source}, actual={actual}, ratio={ratio:.4}");
        assert!(
            (ratio - 1.0).abs() < 0.02,
            "source consumption should match output duration (ratio {ratio:.3})"
        );
    }

    #[test]
    fn test_deck_realtime_resample_playback_rate_480_frame_callback() {
        let input_rate = 44100u32;
        let duration_secs = 10usize;
        let chunk = 480usize;
        let mut deck = new_deck(chunk);
        let frames = input_rate as usize * duration_secs;
        let mut samples = vec![0.0f32; frames * 2];
        for i in 0..frames {
            let s = (2.0 * std::f32::consts::PI * 440.0 * i as f32 / input_rate as f32).sin() * 0.5;
            samples[i * 2] = s;
            samples[i * 2 + 1] = s;
        }

        load_test_samples(&mut deck, samples, input_rate);
        deck.play().unwrap();

        let total_output_frames = (ENGINE_RATE as usize * duration_secs / chunk) * chunk;
        let mut produced = 0usize;
        while produced < total_output_frames {
            deck.process(chunk).unwrap();
            produced += chunk;
        }

        let expected_source =
            (total_output_frames as f64 * input_rate as f64 / ENGINE_RATE as f64) as u64;
        let actual = deck.position_frames();
        let ratio = actual as f64 / expected_source as f64;
        assert!(
            (ratio - 1.0).abs() < 0.02,
            "480-frame callbacks must not over-consume source (ratio {ratio:.3})"
        );
    }

    #[test]
    fn prepared_load_mismatched_rate_leaves_deck_untouched_until_apply() {
        let input_rate = 44100u32;
        let duration_secs = 10usize;
        let mut deck = new_deck(CHUNK);
        let audio = Arc::new(LoadedAudio {
            samples: vec![0.1f32; input_rate as usize * 2 * duration_secs],
            sample_rate: input_rate,
            channels: 2,
            source_id: "test.wav".to_string(),
        });

        let prepared = deck.load_spec().prepare(audio).unwrap();
        assert!(
            deck.loaded_audio().is_none(),
            "prepare must not mutate deck state"
        );

        deck.apply_prepared_load(prepared);
        assert!(deck.loaded_audio().is_some());
        deck.play().unwrap();

        let total_output_frames = (ENGINE_RATE as usize * duration_secs / CHUNK) * CHUNK;
        let mut produced = 0usize;
        while produced < total_output_frames {
            deck.process(CHUNK).unwrap();
            produced += CHUNK;
        }
        let expected_source =
            (total_output_frames as f64 * input_rate as f64 / ENGINE_RATE as f64) as u64;
        let actual = deck.position_frames();
        let ratio = actual as f64 / expected_source as f64;
        assert!(
            (ratio - 1.0).abs() < 0.02,
            "prepared resampler must convert 44.1k source to engine rate (ratio {ratio:.3})"
        );
    }

    #[test]
    fn prepared_load_matching_rate_consumes_source_one_to_one() {
        let mut deck = new_deck(CHUNK);
        let audio = Arc::new(LoadedAudio {
            samples: vec![0.1f32; CHUNK * 2 * 4],
            sample_rate: ENGINE_RATE,
            channels: 2,
            source_id: "test.wav".to_string(),
        });

        let prepared = deck.load_spec().prepare(audio).unwrap();
        deck.apply_prepared_load(prepared);
        deck.play().unwrap();
        deck.process(CHUNK).unwrap();

        assert_eq!(deck.position_frames() as usize, CHUNK);
    }

    #[test]
    fn prepared_load_key_lock_builds_stretcher_and_advances_at_tempo() {
        let mut vinyl = new_deck(CHUNK);
        let mut locked = new_deck(CHUNK);
        let samples = vec![0.25f32; CHUNK * 2 * 500];
        load_test_samples(&mut vinyl, samples.clone(), ENGINE_RATE);

        locked.set_key_lock(true).unwrap();
        let audio = Arc::new(LoadedAudio {
            samples,
            sample_rate: ENGINE_RATE,
            channels: 2,
            source_id: "test.wav".to_string(),
        });
        let prepared = locked.load_spec().prepare(audio).unwrap();
        locked.apply_prepared_load(prepared);
        assert!(
            locked.stretcher.is_some(),
            "key-lock prepared load must install a stretcher"
        );

        vinyl.set_speed(0.0).unwrap();
        locked.set_speed(0.0).unwrap();
        vinyl.play().unwrap();
        locked.play().unwrap();

        for _ in 0..32 {
            locked.process(CHUNK).unwrap();
            vinyl.process(CHUNK).unwrap();
        }
        let locked_start = locked.position_frames();
        let vinyl_start = vinyl.position_frames();
        for _ in 0..64 {
            vinyl.process(CHUNK).unwrap();
            locked.process(CHUNK).unwrap();
        }
        let locked_delta = locked.position_frames() - locked_start;
        let vinyl_delta = vinyl.position_frames() - vinyl_start;
        assert!(
            locked_delta > CHUNK as i64 * 32,
            "key lock should advance faster than 1x: delta={locked_delta}"
        );
        assert!(
            vinyl_delta > CHUNK as i64 * 32,
            "vinyl should advance faster than 1x: delta={vinyl_delta}"
        );
    }

    #[test]
    fn test_deck_resample_advances_when_process_uses_graph_chunk() {
        // Engine device buffer is often 512; mixer always asks for Buffer::LEN (64).
        // Rubato FixedSync::Output sized to 512 returns (0,0) into a 64-frame out buf.
        let mut deck = new_deck(CHUNK);
        let frames = 44100usize;
        let mut samples = vec![0.0f32; frames * 2];
        for i in 0..frames {
            let s = (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 44100.0).sin() * 0.5;
            samples[i * 2] = s;
            samples[i * 2 + 1] = s;
        }
        load_test_samples(&mut deck, samples, 44100);
        deck.play().unwrap();

        let mut heard = false;
        for _ in 0..8 {
            let audio = deck.process(Buffer::LEN).unwrap();
            heard |= audio.iter().any(|&s| s.abs() > 0.01);
        }
        assert!(
            heard,
            "graph-sized process must produce audio after 44.1→48k resample"
        );
        assert!(
            deck.position_frames() > 0,
            "source position must advance (got {})",
            deck.position_frames()
        );
    }

    #[test]
    fn test_deck_realtime_resample_produces_audio() {
        let mut deck = new_deck(CHUNK);
        let frames = 44100usize;
        let mut samples = vec![0.0f32; frames * 2];
        for i in 0..frames {
            let s = (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 44100.0).sin() * 0.5;
            samples[i * 2] = s;
            samples[i * 2 + 1] = s;
        }

        load_test_samples(&mut deck, samples, 44100);
        deck.play().unwrap();

        let mut non_zero = 0usize;
        for _ in 0..200 {
            let audio = deck.process(CHUNK).unwrap();
            non_zero += audio.iter().filter(|&&s| s.abs() > 0.001).count();
        }

        assert!(
            non_zero > 0,
            "realtime resample should produce non-zero audio within 200 callbacks"
        );
    }

    #[test]
    fn test_deck_shares_loaded_audio_arc() {
        let audio = Arc::new(LoadedAudio {
            samples: vec![0.5f32; CHUNK * 2],
            sample_rate: ENGINE_RATE,
            channels: 2,
            source_id: "test.wav".to_string(),
        });

        let mut deck_a = new_deck(CHUNK);
        let mut deck_b = new_deck(CHUNK);
        deck_a.load(Arc::clone(&audio)).unwrap();
        deck_b.load(Arc::clone(&audio)).unwrap();

        assert!(Arc::ptr_eq(
            deck_a.loaded_audio().unwrap(),
            deck_b.loaded_audio().unwrap()
        ));
    }

    #[test]
    fn test_deck_pause_outputs_silence_not_stale_buffer() {
        let mut deck = new_deck(CHUNK);
        load_test_samples(&mut deck, vec![0.8f32; CHUNK * 2], ENGINE_RATE);
        deck.play().unwrap();

        let playing = deck.process(CHUNK).unwrap();
        assert!(playing.iter().any(|&s| s.abs() > 0.01));

        deck.pause().unwrap();
        let paused = deck.process(CHUNK).unwrap();
        assert!(
            paused.iter().all(|&s| s == 0.0),
            "paused deck must output silence, got stale samples: max={}",
            paused.iter().map(|s| s.abs()).fold(0.0f32, f32::max)
        );
    }

    #[test]
    fn test_deck_stop_outputs_silence_not_stale_buffer() {
        let mut deck = new_deck(CHUNK);
        load_test_samples(&mut deck, vec![0.8f32; CHUNK * 2], ENGINE_RATE);
        deck.play().unwrap();
        deck.process(CHUNK).unwrap();

        deck.stop().unwrap();
        let stopped = deck.process(CHUNK).unwrap();
        assert!(
            stopped.iter().all(|&s| s == 0.0),
            "stopped deck must output silence, got stale samples: max={}",
            stopped.iter().map(|s| s.abs()).fold(0.0f32, f32::max)
        );
    }

    #[test]
    fn paused_vinyl_touch_release_does_not_nudge_playhead() {
        let mut deck = new_deck(CHUNK);
        load_test_samples(&mut deck, vec![0.5f32; CHUNK * 2 * 5000], ENGINE_RATE);
        deck.play().unwrap();
        deck.pause().unwrap();
        deck.seek_ms(100).unwrap();
        let start = deck.position_ms().unwrap();
        deck.set_jog_touch(true);
        for _ in 0..5 {
            deck.process(CHUNK).unwrap();
        }
        deck.set_jog_touch(false);
        for _ in 0..50 {
            deck.process(CHUNK).unwrap();
        }
        let end = deck.position_ms().unwrap();
        assert_eq!(
            end, start,
            "touch/release without turns must not scrub while paused"
        );
        assert!(
            (deck.jog_rate() - 1.0).abs() < 1e-4,
            "rate={}",
            deck.jog_rate()
        );
        assert!(!deck.jog_driving_audio());
    }

    #[test]
    fn paused_vinyl_jog_advances_position_ms() {
        let mut deck = new_deck(CHUNK);
        load_test_samples(&mut deck, vec![0.5f32; CHUNK * 2 * 5000], ENGINE_RATE);
        deck.play().unwrap();
        deck.pause().unwrap();
        assert_eq!(deck.state(), &DeckState::Paused);
        let start = deck.position_ms().unwrap();
        deck.set_jog_touch(true);
        for _ in 0..40 {
            let _ = deck.process(48);
            deck.jog_turn(20);
            let _ = deck.process(CHUNK);
        }
        let end = deck.position_ms().unwrap();
        assert!(
            (end - start).abs() >= 5,
            "paused vinyl jog should move playhead, start={start} end={end}"
        );
    }

    #[test]
    fn vinyl_touch_holds_at_zero_rate() {
        let mut deck = new_deck(CHUNK);
        load_test_samples(&mut deck, vec![0.5f32; CHUNK * 2 * 100], ENGINE_RATE);
        deck.play().unwrap();
        deck.set_jog_touch(true);
        assert_eq!(deck.jog_rate(), 0.0);
        deck.process(CHUNK).unwrap();
        assert_eq!(deck.jog_rate(), 0.0);
    }

    #[test]
    fn vinyl_turn_moves_rate_and_release_ramps() {
        let mut deck = new_deck(CHUNK);
        load_test_samples(&mut deck, vec![0.5f32; CHUNK * 2 * 100], ENGINE_RATE);
        deck.set_jog_touch(true);
        for _ in 0..8 {
            let _ = deck.process(48);
            deck.jog_turn(8);
        }
        assert!(deck.jog_rate().abs() > 0.05, "rate={}", deck.jog_rate());
        deck.set_jog_touch(false);
        for _ in 0..50 {
            deck.process(CHUNK).unwrap();
        }
        assert!(
            (deck.jog_rate() - 1.0).abs() < 0.05,
            "expected ramp to 1, got {}",
            deck.jog_rate()
        );
    }

    #[test]
    fn outer_pitch_bend_decays() {
        let mut deck = new_deck(CHUNK);
        load_test_samples(&mut deck, vec![0.5f32; CHUNK * 2 * 100], ENGINE_RATE);
        deck.play().unwrap();
        assert!(!deck.jog_touching());
        assert_eq!(deck.outer_jog_mode(), JogMode::PitchBend);
        deck.jog_turn(40);
        deck.process(CHUNK).unwrap();
        assert!((deck.jog_rate() - 1.0).abs() > 1e-3);
        for _ in 0..100 {
            deck.process(CHUNK).unwrap();
        }
        assert!((deck.jog_rate() - 1.0).abs() < 1e-3);
    }

    #[test]
    fn paused_outer_jog_scrubs_not_pitch_bend() {
        let mut deck = new_deck(CHUNK);
        load_test_samples(&mut deck, vec![0.5f32; CHUNK * 2 * 5000], ENGINE_RATE);
        deck.play().unwrap();
        deck.pause().unwrap();
        let start = deck.position_ms().unwrap();
        assert!(!deck.jog_touching());
        for _ in 0..40 {
            deck.jog_turn(20);
            deck.process(CHUNK).unwrap();
        }
        let end = deck.position_ms().unwrap();
        assert!(
            (end - start).abs() >= 5,
            "paused outer jog should scrub playhead, start={start} end={end}"
        );
        assert!(
            deck.jog_rate().abs() > 0.15,
            "paused outer must use absolute scrub rate, got {}",
            deck.jog_rate()
        );
    }

    #[test]
    fn ignore_mode_is_noop() {
        let mut deck = new_deck(CHUNK);
        deck.set_jog_mode(JogMode::Ignore, JogMode::Ignore);
        deck.set_jog_touch(true);
        deck.jog_turn(50);
        assert_eq!(deck.jog_rate(), 1.0);
    }

    #[test]
    fn key_lock_advances_faster_than_unity_at_positive_pitch() {
        let mut vinyl = new_deck(CHUNK);
        let mut locked = new_deck(CHUNK);
        let samples = vec![0.25f32; CHUNK * 2 * 500];
        load_test_samples(&mut vinyl, samples.clone(), ENGINE_RATE);
        load_test_samples(&mut locked, samples, ENGINE_RATE);

        vinyl.set_speed(0.0).unwrap(); // +tempo_range → faster
        locked.set_speed(0.0).unwrap();
        locked.set_key_lock(true).unwrap();
        assert!(locked.key_lock());

        vinyl.play().unwrap();
        locked.play().unwrap();

        // Warm up stretcher latency.
        for _ in 0..32 {
            locked.process(CHUNK).unwrap();
        }
        let locked_start = locked.position_frames();
        let vinyl_start = {
            for _ in 0..32 {
                vinyl.process(CHUNK).unwrap();
            }
            vinyl.position_frames()
        };

        for _ in 0..64 {
            vinyl.process(CHUNK).unwrap();
            locked.process(CHUNK).unwrap();
        }

        let vinyl_delta = vinyl.position_frames() - vinyl_start;
        let locked_delta = locked.position_frames() - locked_start;
        assert!(
            vinyl_delta > CHUNK as i64 * 32,
            "vinyl should advance faster than 1x: delta={vinyl_delta}"
        );
        assert!(
            locked_delta > CHUNK as i64 * 32,
            "key lock should advance faster than 1x: delta={locked_delta}"
        );
    }

    #[test]
    fn key_lock_falls_back_to_vinyl_while_jog_driving() {
        let mut deck = new_deck(CHUNK);
        load_test_samples(&mut deck, vec![0.4f32; CHUNK * 2 * 8_000], ENGINE_RATE);
        deck.set_key_lock(true).unwrap();
        deck.play().unwrap();
        for _ in 0..8 {
            deck.process(CHUNK).unwrap();
        }

        deck.set_jog_touch(true);
        deck.jog_turn(20);
        assert!(deck.jog_driving_audio());
        let before = deck.position_frames();
        deck.process(CHUNK).unwrap();
        // Vinyl jog path still runs (does not panic / stall) while key lock stays on.
        assert!(deck.key_lock());
        assert!(deck.position_frames() != before || deck.jog_rate() != 1.0);
    }

    #[test]
    fn key_shift_falls_back_to_vinyl_while_jog_driving() {
        let mut deck = new_deck(CHUNK);
        load_test_samples(&mut deck, vec![0.4f32; CHUNK * 2 * 8_000], ENGINE_RATE);
        deck.set_key_shift_semitones(2.0).unwrap();
        deck.play().unwrap();
        for _ in 0..8 {
            deck.process(CHUNK).unwrap();
        }
        assert!(deck.stretch_active, "key shift should stretch at rest");

        deck.set_jog_touch(true);
        deck.jog_turn(20);
        assert!(deck.jog_driving_audio());
        let before = deck.position_frames();
        let out = deck.process(CHUNK).unwrap().to_vec();
        // A latched shift must not block scratching: the vinyl path runs and
        // the playhead moves (no panic / stall).
        assert!(
            !deck.stretch_active,
            "jog must leave the key-shift stretch path"
        );
        assert!(
            deck.position_frames() != before || deck.jog_rate() != 1.0,
            "vinyl jog should drive the playhead"
        );
        assert!(
            out.iter().any(|s| s.abs() > 0.0),
            "scratching must produce audio, not silence"
        );
        assert!((deck.key_shift_semitones() - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn slip_loop_exit_catches_up_to_shadow() {
        let mut deck = new_deck(CHUNK);
        load_test_samples(&mut deck, vec![0.5f32; CHUNK * 2 * 200_000], ENGINE_RATE);
        deck.set_slip_enabled(true).unwrap();
        deck.play().unwrap();
        for _ in 0..20 {
            deck.process(CHUNK).unwrap();
        }
        let in_ms = deck.position_ms().unwrap();
        deck.set_loop_region_ms(in_ms, in_ms + 500).unwrap();
        for _ in 0..200 {
            deck.process(CHUNK).unwrap();
        }
        let looped = deck.position_ms().unwrap();
        let shadow_before = deck.shadow_position_ms().unwrap();
        assert!(
            shadow_before > looped + 50,
            "shadow should outrun loop: shadow={shadow_before} audible={looped}"
        );
        deck.clear_loop();
        let after = deck.position_ms().unwrap();
        assert!(
            (after - shadow_before).abs() <= 5,
            "exit loop should catch up: after={after} shadow={shadow_before}"
        );
        assert!(
            deck.shadow_position_ms().is_none(),
            "exit loop must clear the slip cursor"
        );
    }

    #[test]
    fn slip_scratch_exit_catches_up_to_shadow() {
        let mut deck = new_deck(CHUNK);
        load_test_samples(&mut deck, vec![0.5f32; CHUNK * 2 * 200_000], ENGINE_RATE);
        deck.set_slip_enabled(true).unwrap();
        deck.play().unwrap();
        for _ in 0..10 {
            deck.process(CHUNK).unwrap();
        }
        let start = deck.position_ms().unwrap();
        deck.set_jog_touch(true);
        assert!(
            (deck.shadow_position_ms().unwrap() - start).abs() <= 5,
            "jog touch must arm the slip cursor at the playhead"
        );
        for _ in 0..30 {
            deck.jog_turn(-10);
            deck.process(CHUNK).unwrap();
        }
        let scratched = deck.position_ms().unwrap();
        assert!(
            scratched < start,
            "scratch should move back: {scratched} vs {start}"
        );
        let shadow = deck.shadow_position_ms().unwrap();
        assert!(
            shadow > scratched + 20,
            "shadow should advance: {shadow} vs {scratched}"
        );
        deck.set_jog_touch(false);
        let after = deck.position_ms().unwrap();
        assert!(
            (after - shadow).abs() <= 10,
            "scratch release should catch up: after={after} shadow={shadow}"
        );
        assert!(
            deck.shadow_position_ms().is_none(),
            "scratch release must clear the slip cursor"
        );
    }

    #[test]
    fn slip_shadow_stays_none_until_a_manual_action_starts() {
        let mut deck = new_deck(CHUNK);
        load_test_samples(&mut deck, vec![0.5f32; CHUNK * 2 * 200_000], ENGINE_RATE);
        deck.set_slip_enabled(true).unwrap();
        assert!(
            deck.shadow_position_ms().is_none(),
            "enabling slip must not arm the cursor"
        );
        deck.play().unwrap();
        for _ in 0..50 {
            deck.process(CHUNK).unwrap();
        }
        assert!(
            deck.shadow_position_ms().is_none(),
            "plain playback must not calculate the slip cursor"
        );
        let pos = deck.position_ms().unwrap();
        deck.seek_ms(pos + 100).unwrap();
        assert!(
            deck.shadow_position_ms().is_none(),
            "permanent seek must not arm the cursor"
        );
    }

    #[test]
    fn slip_shadow_loop_capture_survives_seek_and_clears_at_exit() {
        let mut deck = new_deck(CHUNK);
        load_test_samples(&mut deck, vec![0.5f32; CHUNK * 2 * 200_000], ENGINE_RATE);
        deck.set_slip_enabled(true).unwrap();
        deck.play().unwrap();
        for _ in 0..10 {
            deck.process(CHUNK).unwrap();
        }
        let in_ms = deck.position_ms().unwrap();
        deck.set_loop_region_ms(in_ms, in_ms + 500).unwrap();
        assert!(deck.shadow_position_ms().is_some());
        // The loop is still in flight, so the permanent seek re-arms the cursor
        // at the new position instead of dropping it.
        deck.seek_ms(in_ms + 2000).unwrap();
        let rearmed = deck
            .shadow_position_ms()
            .expect("a live loop keeps its cursor across a seek");
        assert!(
            (rearmed - (in_ms + 2000)).abs() <= 5,
            "re-arm at the seek target: {rearmed} vs {}",
            in_ms + 2000
        );
        deck.clear_loop();
        assert!(
            deck.shadow_position_ms().is_none(),
            "loop exit clears the cursor"
        );
    }

    #[test]
    fn slip_shadow_captures_at_cue_hold_and_clears_on_release() {
        let mut deck = new_deck(CHUNK);
        load_test_samples(&mut deck, vec![0.5f32; CHUNK * 2 * 200_000], ENGINE_RATE);
        deck.set_slip_enabled(true).unwrap();
        deck.play().unwrap();
        for _ in 0..10 {
            deck.process(CHUNK).unwrap();
        }
        let hold_start = deck.position_ms().unwrap();
        deck.set_cue_point_ms(hold_start + 1000).unwrap();
        deck.begin_cue_hold().unwrap();
        let shadow = deck
            .shadow_position_ms()
            .expect("cue hold must arm the slip cursor");
        assert!(
            (shadow - hold_start).abs() <= 5,
            "cursor must capture the pre-hold position: {shadow} vs {hold_start}"
        );
        assert!(
            deck.position_ms().unwrap() >= hold_start + 900,
            "audible must jump to the cue"
        );
        for _ in 0..20 {
            deck.process(CHUNK).unwrap();
        }
        let advanced = deck.shadow_position_ms().unwrap();
        assert!(
            advanced > shadow + 5,
            "cursor must keep advancing during the hold: {shadow} -> {advanced}"
        );
        deck.end_cue_hold().unwrap();
        assert!(
            deck.shadow_position_ms().is_none(),
            "cue-hold release must clear the cursor"
        );
        let after = deck.position_ms().unwrap();
        assert!(
            (after - advanced).abs() <= 10,
            "release must catch up: {after} vs {advanced}"
        );
    }

    #[test]
    fn slip_shadow_does_not_arm_on_paused_pitch_bend_outer() {
        let mut deck = new_deck(CHUNK);
        load_test_samples(&mut deck, vec![0.5f32; CHUNK * 2 * 200_000], ENGINE_RATE);
        deck.set_slip_enabled(true).unwrap();
        // Paused: `active_mode` remaps an outer PitchBend jog to vinyl, so the
        // vinyl-only arm gate must still reject this non-scratch touch.
        deck.set_jog_mode(JogMode::Vinyl, JogMode::PitchBend);
        let start = deck.position_ms().unwrap();
        deck.set_jog_touch(true);
        assert!(
            deck.shadow_position_ms().is_none(),
            "a paused touch must not arm the slip cursor"
        );
        for _ in 0..40 {
            let _ = deck.process(48);
            deck.jog_turn(20);
            let _ = deck.process(CHUNK);
        }
        let scratched = deck.position_ms().unwrap();
        assert!(
            (scratched - start).abs() >= 5,
            "the scratch must actually move the playhead: {start} -> {scratched}"
        );
        deck.set_jog_touch(false);
        assert!(
            deck.shadow_position_ms().is_none(),
            "a paused release must not leave a cursor"
        );
        let after = deck.position_ms().unwrap();
        assert!(
            (after - scratched).abs() <= 5,
            "a paused release must not snap the playhead: {scratched} -> {after}"
        );
    }

    #[test]
    fn slip_shadow_survives_cue_hold_while_loop_active() {
        let mut deck = new_deck(CHUNK);
        load_test_samples(&mut deck, vec![0.5f32; CHUNK * 2 * 200_000], ENGINE_RATE);
        deck.set_slip_enabled(true).unwrap();
        deck.play().unwrap();
        for _ in 0..10 {
            deck.process(CHUNK).unwrap();
        }
        let in_ms = deck.position_ms().unwrap();
        deck.set_loop_region_ms(in_ms, in_ms + 500).unwrap();
        let armed = deck.shadow_position_ms().expect("loop arms the cursor");
        // The cue hold must not steal (or strip) the loop's cursor.
        deck.set_cue_point_ms(in_ms + 1000).unwrap();
        deck.begin_cue_hold().unwrap();
        for _ in 0..20 {
            deck.process(CHUNK).unwrap();
        }
        let held = deck.shadow_position_ms().expect("cursor still armed");
        assert!(
            held > armed,
            "cursor advances during the hold: {armed} -> {held}"
        );
        deck.end_cue_hold().unwrap();
        let after_hold = deck
            .shadow_position_ms()
            .expect("cue-hold release must keep the loop's cursor");
        assert!(
            (after_hold - held).abs() <= 10,
            "the loop's cursor survives the hold at the caught-up position: held={held} after_hold={after_hold}"
        );
        for _ in 0..10 {
            deck.process(CHUNK).unwrap();
        }
        let before_exit = deck
            .shadow_position_ms()
            .expect("cursor still armed after the hold");
        deck.clear_loop();
        let audible = deck.position_ms().unwrap();
        let cursor = deck.shadow_position_ms();
        assert!(
            cursor.is_none(),
            "loop exit after a hold clears the cursor once: {cursor:?}"
        );
        assert!(
            (audible - before_exit).abs() <= 10,
            "loop exit must still catch up: audible={audible} before_exit={before_exit}"
        );
    }

    #[test]
    fn slip_shadow_survives_loop_exit_during_cue_hold() {
        let mut deck = new_deck(CHUNK);
        load_test_samples(&mut deck, vec![0.5f32; CHUNK * 2 * 200_000], ENGINE_RATE);
        deck.set_slip_enabled(true).unwrap();
        deck.play().unwrap();
        for _ in 0..10 {
            deck.process(CHUNK).unwrap();
        }
        let in_ms = deck.position_ms().unwrap();
        deck.set_loop_region_ms(in_ms, in_ms + 500).unwrap();
        let armed = deck.shadow_position_ms().expect("loop arms the cursor");
        deck.set_cue_point_ms(in_ms + 1000).unwrap();
        deck.begin_cue_hold().unwrap();
        for _ in 0..20 {
            deck.process(CHUNK).unwrap();
        }
        // Loop exits while the cue hold is still held: the hold keeps a cursor.
        deck.clear_loop();
        let after_exit = deck
            .shadow_position_ms()
            .expect("loop exit must keep the cue hold's cursor");
        assert!(
            after_exit > armed,
            "re-armed cursor continues advancing: {armed} -> {after_exit}"
        );
        deck.end_cue_hold().unwrap();
        assert!(
            deck.shadow_position_ms().is_none(),
            "cue-hold release clears the cursor once nothing else needs it"
        );
    }

    #[test]
    fn slip_shadow_mode_switch_clears_touch_cursor_only() {
        let mut deck = new_deck(CHUNK);
        load_test_samples(&mut deck, vec![0.5f32; CHUNK * 2 * 200_000], ENGINE_RATE);
        deck.set_slip_enabled(true).unwrap();
        deck.play().unwrap();
        for _ in 0..10 {
            deck.process(CHUNK).unwrap();
        }
        // Touch-armed: a mid-touch mode switch abandons the scratch.
        deck.set_jog_touch(true);
        assert!(
            deck.shadow_position_ms().is_some(),
            "touch arms the cursor while playing"
        );
        // A mid-touch switch that actually leaves vinyl abandons the scratch
        // (the motion starts a release ramp), so its cursor is dropped.
        deck.set_jog_mode(JogMode::PitchBend, JogMode::Vinyl);
        assert!(
            deck.shadow_position_ms().is_none(),
            "a mid-touch switch off vinyl must drop a touch-armed cursor"
        );
        deck.set_jog_touch(false);
        // Loop-armed: the loop still owns its cursor across the same switch.
        let in_ms = deck.position_ms().unwrap();
        deck.set_loop_region_ms(in_ms, in_ms + 500).unwrap();
        let armed = deck.shadow_position_ms().expect("loop arms the cursor");
        deck.set_jog_touch(true);
        deck.set_jog_mode(JogMode::PitchBend, JogMode::Vinyl);
        assert!(
            deck.shadow_position_ms().is_some(),
            "a mid-touch switch must keep a loop-armed cursor"
        );
        deck.set_jog_touch(false);
        deck.clear_loop();
        let audible = deck.position_ms().unwrap();
        assert!(
            (audible - armed).abs() <= 30,
            "loop exit must still catch up: audible={audible} armed={armed}"
        );
    }

    #[test]
    fn slip_shadow_survives_vinyl_to_vinyl_mode_switch() {
        let mut deck = new_deck(CHUNK);
        load_test_samples(&mut deck, vec![0.5f32; CHUNK * 2 * 200_000], ENGINE_RATE);
        deck.set_slip_enabled(true).unwrap();
        deck.play().unwrap();
        for _ in 0..10 {
            deck.process(CHUNK).unwrap();
        }
        deck.set_jog_touch(true);
        let armed = deck.shadow_position_ms().expect("touch arms while playing");
        // Same active mode: `set_modes` starts no release ramp, the finger is
        // still on a running platter and the scratch continues.
        deck.set_jog_mode(JogMode::Vinyl, JogMode::Vinyl);
        assert!(
            deck.shadow_position_ms().is_some(),
            "a vinyl→vinyl switch must not drop a live scratch cursor"
        );
        for _ in 0..20 {
            deck.process(CHUNK).unwrap();
        }
        let advanced = deck.shadow_position_ms().expect("cursor keeps advancing");
        assert!(
            advanced > armed + 5,
            "cursor advances through the switch: {armed} -> {advanced}"
        );
        deck.set_jog_touch(false);
        let after = deck.position_ms().unwrap();
        assert!(
            (after - advanced).abs() <= 10,
            "the real release catches up: {after} vs {advanced}"
        );
        assert!(deck.shadow_position_ms().is_none());
    }

    #[test]
    fn slip_shadow_does_not_arm_for_paused_jog_at_loop_exit() {
        let mut deck = new_deck(CHUNK);
        load_test_samples(&mut deck, vec![0.5f32; CHUNK * 2 * 200_000], ENGINE_RATE);
        deck.set_slip_enabled(true).unwrap();
        deck.play().unwrap();
        for _ in 0..10 {
            deck.process(CHUNK).unwrap();
        }
        deck.pause().unwrap();
        deck.set_jog_touch(true);
        for _ in 0..30 {
            deck.jog_turn(-10);
            deck.process(CHUNK).unwrap();
        }
        let scratched = deck.position_ms().unwrap();
        // A loop exit with no region must not resurrect a cursor for a paused
        // jog: arming one here freezes it while the playhead moves, and the
        // release then snaps the scratch back to the arm point.
        deck.clear_loop();
        assert!(
            deck.shadow_position_ms().is_none(),
            "a paused jog must not gain a cursor at loop exit"
        );
        deck.set_jog_touch(false);
        let after = deck.position_ms().unwrap();
        assert!(
            (after - scratched).abs() <= 10,
            "the paused scratch must survive the release: {scratched} -> {after}"
        );
    }

    #[test]
    fn slip_shadow_arms_when_slip_enabled_during_active_loop() {
        let mut deck = new_deck(CHUNK);
        load_test_samples(&mut deck, vec![0.5f32; CHUNK * 2 * 200_000], ENGINE_RATE);
        // Loop first, slip second: the loop is already in flight.
        deck.play().unwrap();
        for _ in 0..10 {
            deck.process(CHUNK).unwrap();
        }
        let in_ms = deck.position_ms().unwrap();
        deck.set_loop_region_ms(in_ms, in_ms + 500).unwrap();
        deck.set_slip_enabled(true).unwrap();
        let armed = deck
            .shadow_position_ms()
            .expect("enabling slip during a live loop arms the cursor");
        assert!(
            (armed - in_ms).abs() <= 5,
            "arm at the current position: {armed} vs {in_ms}"
        );
        for _ in 0..40 {
            deck.process(CHUNK).unwrap();
        }
        let before_exit = deck.shadow_position_ms().expect("cursor advances");
        deck.clear_loop();
        let audible = deck.position_ms().unwrap();
        assert!(
            (audible - before_exit).abs() <= 10,
            "loop exit catches up: {audible} vs {before_exit}"
        );
    }

    #[test]
    fn slip_shadow_does_not_rearm_on_paused_seek() {
        let mut deck = new_deck(CHUNK);
        load_test_samples(&mut deck, vec![0.5f32; CHUNK * 2 * 200_000], ENGINE_RATE);
        deck.set_slip_enabled(true).unwrap();
        deck.play().unwrap();
        for _ in 0..10 {
            deck.process(CHUNK).unwrap();
        }
        let in_ms = deck.position_ms().unwrap();
        deck.set_loop_region_ms(in_ms, in_ms + 500).unwrap();
        deck.pause().unwrap();
        deck.set_jog_touch(true);
        for _ in 0..30 {
            deck.jog_turn(-10);
            deck.process(CHUNK).unwrap();
        }
        // A permanent seek on a paused deck must not re-anchor a cursor: it
        // would freeze while the scratch keeps moving the playhead, and the
        // next catch-up would snap the scratch back to the stale seek target.
        let target = deck.position_ms().unwrap() - 500;
        deck.seek_ms(target).unwrap();
        assert!(
            deck.shadow_position_ms().is_none(),
            "a paused deck must not re-arm the cursor on a permanent seek"
        );
        deck.set_jog_touch(false);
        let after = deck.position_ms().unwrap();
        assert!(
            (after - target).abs() <= 10,
            "the scratch must stay where the seek left it: {target} -> {after}"
        );
    }

    #[test]
    fn slip_shadow_survives_permanent_seek_during_loop() {
        let mut deck = new_deck(CHUNK);
        load_test_samples(&mut deck, vec![0.5f32; CHUNK * 2 * 200_000], ENGINE_RATE);
        deck.set_slip_enabled(true).unwrap();
        deck.play().unwrap();
        for _ in 0..10 {
            deck.process(CHUNK).unwrap();
        }
        let in_ms = deck.position_ms().unwrap();
        deck.set_loop_region_ms(in_ms, in_ms + 500).unwrap();
        // A permanent seek relocates the deck inside the live region; the loop
        // is still in flight, so it keeps a cursor at the new position.
        let target = in_ms + 120;
        deck.seek_ms(target).unwrap();
        let rearmed = deck
            .shadow_position_ms()
            .expect("a live loop keeps its cursor across a seek");
        assert!(
            (rearmed - target).abs() <= 5,
            "re-arm at the seek target: {rearmed} vs {target}"
        );
        for _ in 0..60 {
            deck.process(CHUNK).unwrap();
        }
        let before_exit = deck.shadow_position_ms().expect("cursor advanced");
        deck.clear_loop();
        let audible = deck.position_ms().unwrap();
        assert!(
            deck.shadow_position_ms().is_none(),
            "loop exit clears the cursor once its own catch-up landed"
        );
        assert!(
            (audible - before_exit).abs() <= 10,
            "exit catches up from the seek point: audible={audible} before_exit={before_exit}"
        );
    }

    #[test]
    fn stems_clear_stretch_active_when_key_lock_on() {
        let mut deck = new_deck(CHUNK);
        load_test_samples(&mut deck, vec![0.3f32; CHUNK * 2 * 8_000], ENGINE_RATE);
        deck.set_key_lock(true).unwrap();
        deck.play().unwrap();
        for _ in 0..4 {
            deck.process(CHUNK).unwrap();
        }
        assert!(deck.stretch_active);

        let stem = Arc::new(LoadedAudio {
            samples: vec![0.5f32; CHUNK * 2 * 8_000],
            sample_rate: 44_100,
            channels: 2,
            source_id: "stem.wav".into(),
        });
        deck.attach_stems([
            Arc::clone(&stem),
            Arc::clone(&stem),
            Arc::clone(&stem),
            Arc::clone(&stem),
        ]);
        deck.process(CHUNK).unwrap();
        assert!(
            !deck.stretch_active,
            "stems must clear stretch_active even with key lock"
        );
    }

    #[test]
    fn mix_at_position_scales_stem_rate_to_main() {
        let mut deck = new_deck(CHUNK);
        // Main track: 2 frames at 48 kHz → silence elsewhere.
        let main = vec![1.0f32, 1.0, 0.0, 0.0];
        load_test_samples(&mut deck, main, 48_000);

        // Stem at 24 kHz: impulse at frame 0 only. At main pos=2.0, stem pos should be 1.0.
        let mut stem_samples = vec![0.0f32; 8];
        stem_samples[0] = 0.25;
        stem_samples[1] = 0.25;
        stem_samples[2] = 0.75;
        stem_samples[3] = 0.75;
        let stem = Arc::new(LoadedAudio {
            samples: stem_samples,
            sample_rate: 24_000,
            channels: 2,
            source_id: "stem.wav".into(),
        });
        deck.attach_stems([
            Arc::clone(&stem),
            Arc::clone(&stem),
            Arc::clone(&stem),
            Arc::clone(&stem),
        ]);

        let (l, _) = deck.mix_at_position(2.0, &[], 48_000);
        // Four stems * 0.75 at stem frame 1.
        assert!((l - 3.0).abs() < 1e-5, "scaled stem sample={l}");
    }

    #[test]
    fn stems_ni_index_order_mute_zero_silences_drums() {
        // NI attach order: drums (0), bass (1), other (2), vocals (3).
        let mut deck = new_deck(CHUNK);
        load_test_samples(&mut deck, vec![0.0f32; CHUNK * 2], ENGINE_RATE);

        fn stem_with_level(level: f32) -> Arc<LoadedAudio> {
            Arc::new(LoadedAudio {
                samples: vec![level; CHUNK * 2 * 4],
                sample_rate: ENGINE_RATE,
                channels: 2,
                source_id: "stem.wav".into(),
            })
        }

        deck.attach_stems([
            stem_with_level(0.4),
            stem_with_level(0.3),
            stem_with_level(0.2),
            stem_with_level(0.1),
        ]);
        deck.stem_gain_current.fill(1.0);

        let (full, _) = deck.mix_at_position(0.0, &[], ENGINE_RATE);
        assert!(
            (full - 1.0).abs() < 1e-5,
            "all four stems audible at pos 0, got {full}"
        );

        deck.set_stem_mute(0, true);
        deck.stem_gain_current[0] = 0.0;
        let (without_drums, _) = deck.mix_at_position(0.0, &[], ENGINE_RATE);
        assert!(
            (without_drums - 0.6).abs() < 1e-5,
            "index 0 is drums: mute removes 0.4, got {without_drums}"
        );

        deck.set_stem_mute(0, false);
        deck.set_stem_mute(3, true);
        for i in 0..4 {
            deck.stem_gain_current[i] = deck.stem_target_gain(i);
        }
        let (without_vocals, _) = deck.mix_at_position(0.0, &[], ENGINE_RATE);
        assert!(
            (without_vocals - 0.9).abs() < 1e-5,
            "index 3 is vocals: mute removes 0.1, got {without_vocals}"
        );
    }

    #[test]
    fn stem_mute_ramps_gain_instead_of_stepping() {
        let mut deck = new_deck(CHUNK);
        load_test_samples(&mut deck, vec![1.0f32; CHUNK * 2 * 8_000], ENGINE_RATE);
        let stem = Arc::new(LoadedAudio {
            samples: vec![1.0f32; CHUNK * 2 * 8_000],
            sample_rate: ENGINE_RATE,
            channels: 2,
            source_id: "stem.wav".into(),
        });
        deck.attach_stems([
            Arc::clone(&stem),
            Arc::clone(&stem),
            Arc::clone(&stem),
            Arc::clone(&stem),
        ]);
        deck.set_stem_mute(0, true);
        assert_eq!(deck.stem_target_gain(0), 0.0);
        assert_eq!(deck.stem_gain_current[0], 1.0);
        deck.ramp_stem_gains_one_sample();
        assert!(
            deck.stem_gain_current[0] < 1.0 && deck.stem_gain_current[0] > 0.0,
            "gain should be mid-ramp, got {}",
            deck.stem_gain_current[0]
        );
    }

    fn brake_test_deck() -> Deck {
        let mut deck = new_deck(CHUNK);
        let frames = ENGINE_RATE as usize * 4;
        let samples: Vec<Sample> = (0..frames)
            .flat_map(|i| {
                let v = (i as f32 * 0.05).sin() * 0.5;
                [v, v]
            })
            .collect();
        load_test_samples(&mut deck, samples, ENGINE_RATE);
        deck.set_vinyl_brake(true, 0.5);
        deck.play().unwrap();
        for _ in 0..4 {
            deck.process(CHUNK).unwrap();
        }
        deck
    }

    #[test]
    fn vinyl_brake_spins_down_over_stop_secs_on_pause() {
        let mut deck = brake_test_deck();
        deck.pause().unwrap();
        assert_eq!(deck.state, DeckState::Paused);

        let brake_frames = ENGINE_RATE as usize / 2;
        let mut elapsed = 0usize;
        while elapsed < brake_frames {
            assert!(
                deck.jog_driving_audio(),
                "brake must drive audio at elapsed {elapsed}"
            );
            deck.process(CHUNK).unwrap();
            elapsed += CHUNK;
        }
        assert!(!deck.jog_driving_audio(), "brake must end after stop_secs");
        let out = deck.process(CHUNK).unwrap();
        assert!(out.iter().all(|s| *s == 0.0), "output silent after brake");
    }

    #[test]
    fn vinyl_brake_play_cancels_brake_instantly() {
        let mut deck = brake_test_deck();
        deck.pause().unwrap();
        deck.process(CHUNK).unwrap();
        assert!(deck.brake.is_some());
        deck.play().unwrap();
        assert!(deck.brake.is_none());
        assert!((deck.motion.rate() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn vinyl_brake_load_cancels_active_brake() {
        let mut deck = brake_test_deck();
        deck.pause().unwrap();
        deck.process(CHUNK).unwrap();
        assert!(deck.brake.is_some(), "brake active before load");

        load_test_samples(&mut deck, vec![0.25f32; CHUNK * 4], ENGINE_RATE);

        assert!(deck.brake.is_none(), "load must drop the brake");
        assert!(!deck.jog_driving_audio());
        assert_eq!(deck.state, DeckState::Paused);
        let out = deck.process(CHUNK).unwrap();
        assert!(out.iter().all(|s| *s == 0.0), "silent while paused");
        assert_eq!(deck.position_frames(), 0, "playhead must not advance");
    }

    #[test]
    fn vinyl_brake_unload_cancels_active_brake() {
        let mut deck = brake_test_deck();
        deck.pause().unwrap();
        deck.process(CHUNK).unwrap();
        assert!(deck.brake.is_some(), "brake active before unload");

        deck.unload().unwrap();

        assert!(deck.brake.is_none(), "unload must drop the brake");
        assert!(!deck.jog_driving_audio());
        let out = deck.process(CHUNK).unwrap();
        assert!(out.iter().all(|s| *s == 0.0), "silent with nothing loaded");
    }

    #[test]
    fn vinyl_brake_starts_when_platter_released_after_paused_touch() {
        let mut deck = brake_test_deck();
        // Pause while the finger owns the platter: no brake may start yet.
        deck.set_jog_touch(true);
        deck.pause().unwrap();
        assert!(deck.brake.is_none(), "touch suppresses the brake");
        deck.jog_turn(64);
        deck.process(CHUNK).unwrap();
        assert!(
            deck.jog_driving_audio(),
            "scratch drives audio while the finger holds the platter"
        );

        deck.set_jog_touch(false);

        assert!(
            deck.brake.is_some(),
            "releasing a paused platter must start the brake"
        );
        assert_eq!(deck.state, DeckState::Paused);
        let mut elapsed = 0usize;
        while elapsed < ENGINE_RATE as usize / 2 {
            assert!(
                deck.jog_driving_audio(),
                "brake must drive audio at elapsed {elapsed}"
            );
            deck.process(CHUNK).unwrap();
            elapsed += CHUNK;
        }
        assert!(!deck.jog_driving_audio(), "brake ends after stop_secs");
    }

    #[test]
    fn vinyl_brake_paused_seek_drops_brake_and_stays_silent() {
        let mut deck = brake_test_deck();
        deck.pause().unwrap();
        deck.process(CHUNK).unwrap();
        assert!(deck.brake.is_some());

        deck.seek_ms(1_000).unwrap();

        assert!(deck.brake.is_none(), "seek drops the brake");
        assert!(!deck.jog_driving_audio(), "paused deck is idle after seek");
        let out = deck.process(CHUNK).unwrap();
        assert!(out.iter().all(|s| *s == 0.0), "silent while paused");
        assert!(
            (deck.position_frames() - ENGINE_RATE as i64).abs() <= 1,
            "seek landed at 1 s, got {}",
            deck.position_frames()
        );
    }

    #[test]
    fn vinyl_brake_ramps_across_multiple_blocks_even_when_window_is_tiny() {
        // 4096-frame block on a 0.05 s window (2400 frames): one block is longer
        // than the whole spin-down, so the ramp must be widened to at least two
        // blocks instead of completing on the first tick and muting instantly.
        const BLOCK: usize = 4096;
        let mut deck = new_deck(BLOCK);
        let frames = ENGINE_RATE as usize * 2;
        let samples: Vec<Sample> = (0..frames)
            .flat_map(|i| {
                let v = (i as f32 * 0.05).sin() * 0.5;
                [v, v]
            })
            .collect();
        load_test_samples(&mut deck, samples, ENGINE_RATE);
        deck.set_vinyl_brake(true, 0.05);
        deck.play().unwrap();
        deck.process(BLOCK).unwrap();
        deck.pause().unwrap();

        let mut rates = Vec::new();
        while deck.brake.is_some() {
            rates.push(deck.jog_rate());
            deck.process(BLOCK).unwrap();
        }
        assert!(
            rates.len() >= 2,
            "brake must span at least two blocks, got {} rate(s): {rates:?}",
            rates.len()
        );
        assert!(
            rates.last().copied().unwrap_or(1.0) < 0.75,
            "a rendered block must already be spinning down, got {rates:?}"
        );
    }

    #[test]
    fn slip_shadow_tracks_audible_through_brake_spin_down() {
        let mut deck = brake_test_deck();
        deck.set_slip_enabled(true).unwrap();
        let in_ms = deck.position_ms().unwrap();
        deck.set_loop_region_ms(in_ms, in_ms + 2000).unwrap();
        assert!(
            deck.shadow_position_ms().is_some(),
            "loop activation must arm the slip cursor"
        );
        let before = deck.position_ms().unwrap();
        deck.pause().unwrap();
        assert!(deck.brake.is_some(), "brake must spin down on pause");
        while deck.brake.is_some() {
            deck.process(CHUNK).unwrap();
        }
        let audible = deck.position_ms().unwrap();
        let shadow = deck.shadow_position_ms().unwrap();
        assert!(
            audible != before,
            "brake must still advance the playhead: before={before} audible={audible}"
        );
        assert!(
            (audible - shadow).abs() <= 5,
            "slip shadow must track the braked playhead: audible={audible} shadow={shadow}"
        );
        deck.clear_loop();
        assert!(
            deck.shadow_position_ms().is_none(),
            "loop exit must clear the slip cursor"
        );
    }

    #[test]
    fn slip_shadow_does_not_drift_after_repeated_braked_stops() {
        let mut deck = brake_test_deck();
        deck.set_slip_enabled(true).unwrap();
        let in_ms = deck.position_ms().unwrap();
        deck.set_loop_region_ms(in_ms, in_ms + 2000).unwrap();
        for _ in 0..3 {
            deck.pause().unwrap();
            assert!(deck.brake.is_some());
            while deck.brake.is_some() {
                deck.process(CHUNK).unwrap();
            }
            deck.play().unwrap();
            for _ in 0..4 {
                deck.process(CHUNK).unwrap();
            }
        }
        let shadow = deck
            .shadow_position_ms()
            .expect("loop keeps the slip cursor armed");
        deck.clear_loop();
        let audible = deck.position_ms().unwrap();
        assert!(
            (audible - shadow).abs() <= 5,
            "repeated stops must not leave the shadow behind: audible={audible} shadow={shadow}"
        );
        assert!(deck.shadow_position_ms().is_none());
    }

    #[test]
    fn slip_shadow_follows_audible_backward_during_negative_brake() {
        let mut deck = brake_test_deck();
        // Headroom so the negative ramp stays inside the track.
        deck.seek_ms(3500).unwrap();
        deck.set_slip_enabled(true).unwrap();
        let in_ms = deck.position_ms().unwrap();
        deck.set_loop_region_ms(in_ms, in_ms + 500).unwrap();
        // A Pad FX envelope is the reachable source of a negative motor rate.
        deck.set_motion_envelope_rate(Some(-1.5));
        deck.pause().unwrap();
        assert!(
            deck.jog_rate() < 0.0,
            "brake must start negative, got {}",
            deck.jog_rate()
        );
        let before_pos = deck.position_frac;
        let before_shadow = deck
            .shadow_position_frac
            .expect("loop activation must arm the slip cursor");
        while deck.brake.is_some() {
            deck.process(CHUNK).unwrap();
        }
        assert!(
            deck.position_frac < before_pos,
            "negative brake must pull the playhead backward: {before_pos} -> {}",
            deck.position_frac
        );
        let after_shadow = deck.shadow_position_frac.expect("cursor still armed");
        assert!(
            after_shadow < before_shadow,
            "shadow must follow the playhead backward: {before_shadow} -> {after_shadow}"
        );
        let audible_delta = deck.position_frac - before_pos;
        let shadow_delta = after_shadow - before_shadow;
        assert!(
            (audible_delta - shadow_delta).abs() < 1.0,
            "shadow must track audible within a frame: audible={audible_delta} shadow={shadow_delta}"
        );
        deck.clear_loop();
        assert!(deck.shadow_position_frac.is_none());
    }
}
