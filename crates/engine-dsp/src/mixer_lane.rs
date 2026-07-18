//! Mixer lane: playback deck + channel strip as one graph node.

use anyhow::Result;
use audio_core::Sample;
use dasp_graph::{Buffer, Input, Node};

use crate::deck::Deck;
use crate::mixer_channel::MixerChannel;

/// One deck slot: owns playback and the strip, renders dry audio then applies the channel.
///
/// Graph role: source-like node (`Lane → Sum`). Before each mixer render, call
/// [`begin_render`](Self::begin_render) so the deck fills a dry stash; each
/// `Node::process` chunk runs that audio through the strip.
#[derive(Debug)]
pub struct MixerLane {
    deck: Deck,
    channel: MixerChannel,
    dry_buffer: Vec<Sample>,
}

impl MixerLane {
    pub fn new(id: usize, sample_rate: u32, buffer_size: u32, resampler_quality: &str) -> Self {
        Self {
            deck: Deck::new(id, sample_rate, buffer_size, resampler_quality),
            channel: MixerChannel::new(sample_rate),
            dry_buffer: Vec::new(),
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

    /// Render dry deck PCM for this callback and reset strip capture state.
    pub fn begin_render(&mut self, frames: u32) -> Result<()> {
        let dry = self.deck.process(frames)?;
        self.dry_buffer.clear();
        self.dry_buffer.extend_from_slice(dry);
        self.channel.begin_render(self.dry_buffer.len());
        Ok(())
    }
}

impl Node for MixerLane {
    fn process(&mut self, _inputs: &[Input], output: &mut [Buffer]) {
        self.channel.process_dry_chunk(&self.dry_buffer, output);
    }
}
