//! Mixer lane: playback deck + sampler + channel strip as one graph node.

use audio_core::Sample;
use dasp_graph::{Buffer, Input, Node};

use crate::deck::Deck;
use crate::mixer_channel::MixerChannel;
use crate::sampler::{Sampler, SamplerStripRoute};

/// One deck slot: owns playback, sampler pads, and the strip.
///
/// Graph role: source-like node (`Lane → Sum`). The mixer clears strip capture once per
/// callback; each `Node::process` renders a full [`Buffer::LEN`] chunk (engine buffer
/// sizes are multiples of that).
#[derive(Debug)]
pub struct MixerLane {
    deck: Deck,
    channel: MixerChannel,
    sampler: Sampler,
    strip_route: SamplerStripRoute,
}

impl MixerLane {
    pub fn new(
        id: usize,
        sample_rate: u32,
        buffer_size: u32,
        resampler_quality: &str,
        strip_route: SamplerStripRoute,
    ) -> Self {
        Self {
            deck: Deck::new(id, sample_rate, buffer_size, resampler_quality),
            channel: MixerChannel::new(sample_rate),
            sampler: Sampler::new(sample_rate, buffer_size, resampler_quality),
            strip_route,
        }
    }

    pub fn deck(&self) -> &Deck {
        &self.deck
    }

    pub fn deck_mut(&mut self) -> &mut Deck {
        &mut self.deck
    }

    pub fn channel(&self) -> &MixerChannel {
        &self.channel
    }

    pub fn channel_mut(&mut self) -> &mut MixerChannel {
        &mut self.channel
    }

    pub fn sampler(&self) -> &Sampler {
        &self.sampler
    }

    pub fn sampler_mut(&mut self) -> &mut Sampler {
        &mut self.sampler
    }

    pub fn strip_route(&self) -> SamplerStripRoute {
        self.strip_route
    }
}

/// Add interleaved `sampler` into planar stereo `output` for one chunk.
fn add_sampler_chunk(sampler: &[Sample], output: &mut [Buffer]) {
    if output.len() < 2 || sampler.len() < 2 {
        return;
    }
    let chunk_frames = (sampler.len() / 2).min(Buffer::LEN);
    let (left_buf, right_buf) = output.split_at_mut(1);
    let left = &mut left_buf[0];
    let right = &mut right_buf[0];
    for frame in 0..chunk_frames {
        let idx = frame * 2;
        left[frame] += sampler[idx];
        right[frame] += sampler[idx + 1];
    }
}

impl Node for MixerLane {
    fn process(&mut self, _inputs: &[Input], output: &mut [Buffer]) {
        const FRAMES: usize = Buffer::LEN;

        let Ok(dry_buffer) = self.deck.process(FRAMES) else {
            for output_buffer in output.iter_mut() {
                *output_buffer = Buffer::SILENT;
            }
            return;
        };

        let mut work_buffer = [0.0_f32; Buffer::LEN * 2];

        match self.strip_route {
            SamplerStripRoute::BeforeStrip => {
                // strip(dry + sampler)
                work_buffer.copy_from_slice(dry_buffer);
                self.sampler.render(FRAMES, &mut work_buffer);
                self.channel.process_dry_chunk(FRAMES, &work_buffer, output);
            }
            SamplerStripRoute::AfterStrip => {
                // strip(dry) + sampler
                self.channel.process_dry_chunk(FRAMES, dry_buffer, output);
                work_buffer.fill(0.0);
                self.sampler.render(FRAMES, &mut work_buffer);
                add_sampler_chunk(&work_buffer, output);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use audio_core::LoadedAudio;
    use std::sync::Arc;

    fn tone_audio() -> Arc<LoadedAudio> {
        let samples = vec![0.5_f32; 48_000 * 2];
        Arc::new(LoadedAudio {
            samples,
            sample_rate: 48_000,
            channels: 2,
            source_id: "tone".into(),
        })
    }

    #[test]
    fn before_strip_sums_in_process() {
        let mut lane = MixerLane::new(0, 48_000, 64, "medium", SamplerStripRoute::BeforeStrip);
        lane.sampler_mut()
            .assign_slot(0, tone_audio(), "t".into(), 48_000, "medium", None)
            .unwrap();
        lane.sampler_mut().trigger(0).unwrap();
        lane.channel_mut().clear_capture();

        let mut outputs = [Buffer::SILENT, Buffer::SILENT];
        lane.process(&[], &mut outputs);
        assert!(outputs[0].iter().any(|s| *s > 0.1));
    }

    #[test]
    fn after_strip_adds_sampler_post_strip() {
        let mut lane = MixerLane::new(0, 48_000, 64, "medium", SamplerStripRoute::AfterStrip);
        lane.channel_mut().set_volume(0.0).unwrap();
        lane.sampler_mut()
            .assign_slot(0, tone_audio(), "t".into(), 48_000, "medium", None)
            .unwrap();
        lane.sampler_mut().trigger(0).unwrap();
        lane.channel_mut().clear_capture();

        let mut outputs = [Buffer::SILENT, Buffer::SILENT];
        lane.process(&[], &mut outputs);
        // Deck path is muted by fader; sampler bypasses strip so output still has energy.
        assert!(outputs[0].iter().any(|s| *s > 0.1));
    }
}
