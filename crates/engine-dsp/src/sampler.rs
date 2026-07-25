//! Sampler bank playback for performance pad mode.
//!
//! Holds up to [`SAMPLER_SLOT_COUNT`] assigned samples and mixes active voices
//! into a stereo buffer with play-mode and loudness auto-gain support.

use anyhow::Result;
use audio_core::{LoadedAudio, Sample};
use std::sync::Arc;

use crate::filter::db_to_linear;
use crate::mixer_channel::AUTO_GAIN_CLAMP_DB;

pub const SAMPLER_SLOT_COUNT: usize = 8;
const MAX_VOICES: usize = 8;

/// Where sampler audio enters the lane relative to the channel strip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SamplerStripRoute {
    /// Mix into dry deck PCM before EQ / filter / fader (default).
    #[default]
    BeforeStrip,
    /// Mix after the strip (still summed with the lane into the crossfader).
    AfterStrip,
}

/// Effective play mode after resolving settings inheritance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SamplerPlayMode {
    #[default]
    Oneshot,
    Hold,
    Loop,
}

/// Metadata for an assigned sampler slot (runtime only).
#[derive(Debug, Clone, Default)]
pub struct SamplerSlotMeta {
    pub label: String,
}

#[derive(Debug, Default)]
struct SamplerSlot {
    audio: Option<Arc<LoadedAudio>>,
    meta: SamplerSlotMeta,
    auto_gain_db: f32,
}

#[derive(Debug)]
struct SamplerVoice {
    audio: Arc<LoadedAudio>,
    slot: usize,
    position: usize,
    gain: f32,
    looping: bool,
    active: bool,
}

impl SamplerVoice {
    fn start(&mut self, slot: usize, audio: Arc<LoadedAudio>, gain: f32, looping: bool) {
        self.audio = audio;
        self.slot = slot;
        self.position = 0;
        self.gain = gain;
        self.looping = looping;
        self.active = true;
    }

    fn stop(&mut self) {
        self.active = false;
        self.position = 0;
        self.looping = false;
    }
}

/// Shared sampler with polyphonic playback and play modes.
#[derive(Debug)]
pub struct Sampler {
    slots: [SamplerSlot; SAMPLER_SLOT_COUNT],
    voices: Vec<SamplerVoice>,
    scratch: Vec<Sample>,
    play_mode: SamplerPlayMode,
    target_lufs: Option<f32>,
}

impl Sampler {
    pub fn new(_sample_rate: u32, _buffer_size: u32, _resampler_quality: &str) -> Self {
        Self {
            slots: std::array::from_fn(|_| SamplerSlot::default()),
            voices: (0..MAX_VOICES)
                .map(|_| SamplerVoice {
                    audio: Arc::new(LoadedAudio {
                        samples: Vec::new(),
                        sample_rate: 48_000,
                        channels: 2,
                        source_id: String::new(),
                    }),
                    slot: 0,
                    position: 0,
                    gain: 1.0,
                    looping: false,
                    active: false,
                })
                .collect(),
            scratch: Vec::new(),
            play_mode: SamplerPlayMode::Oneshot,
            target_lufs: None,
        }
    }

    pub fn play_mode(&self) -> SamplerPlayMode {
        self.play_mode
    }

    pub fn set_play_mode(&mut self, mode: SamplerPlayMode) {
        self.play_mode = mode;
    }

    /// Same normalizer target as decks (`None` = off).
    pub fn set_target_lufs(&mut self, target_lufs: Option<f32>) {
        self.target_lufs = target_lufs.filter(|t| t.is_finite());
        for slot in &mut self.slots {
            // Recompute from stored loudness is not available after assign;
            // callers should re-assign or call set_slot_loudness.
            let _ = slot;
        }
    }

    pub fn set_slot_auto_gain_db(&mut self, slot: usize, auto_gain_db: f32) -> Result<()> {
        let slot = self
            .slots
            .get_mut(slot)
            .ok_or_else(|| anyhow::anyhow!("Invalid sampler slot: {slot}"))?;
        slot.auto_gain_db = auto_gain_db.clamp(-AUTO_GAIN_CLAMP_DB, AUTO_GAIN_CLAMP_DB);
        Ok(())
    }

    pub fn compute_auto_gain_db(&self, loudness_lufs: Option<f64>) -> f32 {
        match (self.target_lufs, loudness_lufs) {
            (Some(target), Some(loudness)) if loudness.is_finite() => {
                (target - loudness as f32).clamp(-AUTO_GAIN_CLAMP_DB, AUTO_GAIN_CLAMP_DB)
            }
            _ => 0.0,
        }
    }

    pub fn slot_meta(&self, slot: usize) -> Option<&SamplerSlotMeta> {
        self.slots
            .get(slot)
            .filter(|s| s.audio.is_some())
            .map(|s| &s.meta)
    }

    pub fn slot_assigned(&self, slot: usize) -> bool {
        self.slots.get(slot).is_some_and(|s| s.audio.is_some())
    }

    pub fn assign_slot(
        &mut self,
        slot: usize,
        audio: Arc<LoadedAudio>,
        label: String,
        engine_sample_rate: u32,
        resampler_quality: &str,
        loudness_lufs: Option<f64>,
    ) -> Result<()> {
        let auto_gain_db = self.compute_auto_gain_db(loudness_lufs);
        let slot = self
            .slots
            .get_mut(slot)
            .ok_or_else(|| anyhow::anyhow!("Invalid sampler slot: {slot}"))?;

        let normalized = normalize_to_engine_rate(audio, engine_sample_rate, resampler_quality)?;
        slot.audio = Some(normalized);
        slot.meta = SamplerSlotMeta { label };
        slot.auto_gain_db = auto_gain_db;
        Ok(())
    }

    pub fn clear_slot(&mut self, slot: usize) -> Result<()> {
        let slot_idx = slot;
        let slot = self
            .slots
            .get_mut(slot)
            .ok_or_else(|| anyhow::anyhow!("Invalid sampler slot: {slot}"))?;
        slot.audio = None;
        slot.meta = SamplerSlotMeta::default();
        slot.auto_gain_db = 0.0;
        for voice in &mut self.voices {
            if voice.active && voice.slot == slot_idx {
                voice.stop();
            }
        }
        Ok(())
    }

    /// Clear all slots (e.g. when switching banks).
    pub fn clear_all_slots(&mut self) {
        for i in 0..SAMPLER_SLOT_COUNT {
            let _ = self.clear_slot(i);
        }
    }

    /// Start playback for a slot according to current play mode.
    pub fn trigger(&mut self, slot: usize) -> Result<()> {
        let (audio, gain) = {
            let s = self
                .slots
                .get(slot)
                .ok_or_else(|| anyhow::anyhow!("Invalid sampler slot: {slot}"))?;
            let audio = s
                .audio
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Sampler slot {slot} is empty"))?
                .clone();
            if audio.samples.is_empty() {
                return Err(anyhow::anyhow!("Sampler slot {slot} has no audio"));
            }
            (audio, db_to_linear(s.auto_gain_db))
        };

        // Hold/loop: stop existing voices for this slot before restarting.
        if matches!(
            self.play_mode,
            SamplerPlayMode::Hold | SamplerPlayMode::Loop
        ) {
            for voice in &mut self.voices {
                if voice.active && voice.slot == slot {
                    voice.stop();
                }
            }
        }

        let looping = self.play_mode == SamplerPlayMode::Loop;
        let voice = if let Some(voice) = self.voices.iter_mut().find(|v| !v.active) {
            voice
        } else {
            self.voices
                .first_mut()
                .expect("voice pool must be non-empty")
        };
        voice.start(slot, audio, gain, looping);
        Ok(())
    }

    /// Stop voices for a slot (hold / loop release).
    pub fn end(&mut self, slot: usize) -> Result<()> {
        if slot >= SAMPLER_SLOT_COUNT {
            return Err(anyhow::anyhow!("Invalid sampler slot: {slot}"));
        }
        for voice in &mut self.voices {
            if voice.active && voice.slot == slot {
                voice.stop();
            }
        }
        Ok(())
    }

    /// Mix active voices into `output` (adds; does not clear).
    pub fn render(&mut self, frames: usize, output: &mut [Sample]) {
        let buffer_size = frames * 2;
        if buffer_size == 0 || output.len() < buffer_size {
            return;
        }

        self.scratch.resize(buffer_size, 0.0);

        for voice in &mut self.voices {
            if !voice.active {
                continue;
            }

            self.scratch.fill(0.0);
            let samples = &voice.audio.samples;
            let channels = 2usize;
            let total = samples.len();
            if total == 0 {
                voice.stop();
                continue;
            }

            let mut out_i = 0usize;
            while out_i < buffer_size {
                if voice.position >= total {
                    if voice.looping {
                        voice.position = 0;
                    } else {
                        voice.stop();
                        break;
                    }
                }
                let available = total - voice.position;
                let to_copy = available.min(buffer_size - out_i);
                self.scratch[out_i..out_i + to_copy]
                    .copy_from_slice(&samples[voice.position..voice.position + to_copy]);
                voice.position += to_copy;
                out_i += to_copy;
                // Align to frame boundary for stereo.
                if to_copy % channels != 0 {
                    break;
                }
            }

            let gain = voice.gain;
            for (out, &sample) in output.iter_mut().zip(self.scratch.iter()) {
                *out += sample * gain;
            }
        }
    }
}

fn normalize_to_engine_rate(
    audio: Arc<LoadedAudio>,
    engine_sample_rate: u32,
    resampler_quality: &str,
) -> Result<Arc<LoadedAudio>> {
    if audio.sample_rate == engine_sample_rate || audio.samples.is_empty() {
        return Ok(audio);
    }

    let channels = audio.channels.max(1) as usize;
    let source_frames = audio.samples.len() / channels;
    if source_frames == 0 {
        return Ok(audio);
    }

    let mut resampler = resampler::create_resampler(
        audio.sample_rate,
        engine_sample_rate,
        channels,
        source_frames.max(512),
        Some(resampler_quality),
    )?;

    let mut out_samples = Vec::new();
    let mut src_pos = 0usize;

    while src_pos < audio.samples.len() {
        let need_in = resampler.input_frames_next();
        let step_out = resampler.output_frames_next();
        let remaining = (audio.samples.len() - src_pos) / channels;
        if remaining == 0 {
            break;
        }

        let in_frames = need_in.min(remaining);
        let chunk = &audio.samples[src_pos..src_pos + in_frames * channels];
        let mut out = vec![0.0f32; step_out * channels];
        let (produced, consumed) = resampler.process(chunk, &mut out, channels);
        if produced == 0 && consumed == 0 {
            break;
        }
        out_samples.extend_from_slice(&out[..produced]);
        src_pos += consumed * channels;
    }

    Ok(Arc::new(LoadedAudio {
        samples: out_samples,
        sample_rate: engine_sample_rate,
        channels: audio.channels,
        source_id: audio.source_id.clone(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_audio(samples: Vec<f32>, sample_rate: u32) -> Arc<LoadedAudio> {
        Arc::new(LoadedAudio {
            samples,
            sample_rate,
            channels: 2,
            source_id: "test".to_string(),
        })
    }

    #[test]
    fn assign_trigger_and_render() {
        let mut sampler = Sampler::new(48_000, 512, "medium");
        let audio = test_audio(vec![0.5, -0.5, 0.25, -0.25], 48_000);
        sampler
            .assign_slot(0, audio, "kick".to_string(), 48_000, "medium", None)
            .unwrap();
        assert!(sampler.slot_assigned(0));

        sampler.trigger(0).unwrap();

        let mut output = vec![0.0; 4];
        sampler.render(2, &mut output);
        assert!(output.iter().any(|&s| s.abs() > 0.0));
    }

    #[test]
    fn hold_end_stops_voice() {
        let mut sampler = Sampler::new(48_000, 512, "medium");
        sampler.set_play_mode(SamplerPlayMode::Hold);
        let audio = test_audio(vec![1.0; 4096], 48_000);
        sampler
            .assign_slot(0, audio, "hit".into(), 48_000, "medium", None)
            .unwrap();
        sampler.trigger(0).unwrap();
        sampler.end(0).unwrap();
        let mut output = vec![0.0; 1024];
        sampler.render(512, &mut output);
        assert!(output.iter().all(|&s| s.abs() < 1e-6));
    }

    #[test]
    fn auto_gain_boosts_quiet_sample() {
        let mut sampler = Sampler::new(48_000, 512, "medium");
        sampler.set_target_lufs(Some(-18.0));
        let audio = test_audio(vec![0.1, 0.1], 48_000);
        sampler
            .assign_slot(0, audio, "quiet".into(), 48_000, "medium", Some(-24.0))
            .unwrap();
        assert!((sampler.slots[0].auto_gain_db - 6.0).abs() < 1e-5);
    }
}
