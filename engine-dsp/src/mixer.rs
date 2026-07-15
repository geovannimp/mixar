//! Audio mixer for routing and mixing multiple decks
//!
//! Implemented with [dasp_graph](https://docs.rs/dasp_graph/latest/dasp_graph/): deck sources
//! feed into a Sum node; output is read from the sum and routed to buses with volume and clamp.

use anyhow::Result;
use audio_core::{slice as audio_slice, BusId, Frame, Sample, StereoFrame};
use dasp_graph::node::Sum;
use dasp_graph::{process, Buffer, Input, Node, NodeData, Processor};
use petgraph::graph::DiGraph;
use std::collections::HashMap;

use crate::deck::{Deck, DeckState};

/// Fixed buffer length used by dasp_graph (samples per channel per process call).
const CHUNK_SAMPLES: usize = Buffer::LEN;

/// Source node that outputs pre-filled buffers (filled by the mixer before each process).
#[derive(Clone, Debug, Default)]
pub struct DeckSourceNode;

impl Node for DeckSourceNode {
    fn process(&mut self, _inputs: &[Input], _output: &mut [Buffer]) {
        // Buffers are filled by the mixer from deck output before process() is called.
    }
}

/// Node type in the mixer graph: either a deck source or the sum.
#[derive(Clone, Debug)]
pub enum MixerNode {
    DeckSource(DeckSourceNode),
    Sum(Sum),
}

impl Node for MixerNode {
    fn process(&mut self, inputs: &[Input], output: &mut [Buffer]) {
        match self {
            MixerNode::DeckSource(n) => n.process(inputs, output),
            MixerNode::Sum(n) => n.process(inputs, output),
        }
    }
}

/// Graph type: directed graph with NodeData<MixerNode>, no edge weight.
type MixerGraph = DiGraph<NodeData<MixerNode>, (), u32>;

/// Audio mixer implemented as a dasp_graph (deck sources -> Sum -> buses).
pub struct Mixer {
    master_volume: f32,
    cue_volume: f32,
    /// Crossfader position: 0.0 = deck A, 1.0 = deck B.
    crossfader: f32,
    /// Internal mix buffer (interleaved stereo) filled from the graph output.
    mix_buffer: Vec<Sample>,
    /// Graph: deck source nodes + sum node.
    graph: MixerGraph,
    /// Processor for running the graph (reused to avoid allocs).
    processor: Processor<MixerGraph>,
    /// Node index for each deck source (graph node id).
    deck_node_ids: Vec<petgraph::graph::NodeIndex>,
    /// Node index for the sum (sink we process to).
    sum_node_id: petgraph::graph::NodeIndex,
}

impl std::fmt::Debug for Mixer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Mixer")
            .field("master_volume", &self.master_volume)
            .field("cue_volume", &self.cue_volume)
            .field("crossfader", &self.crossfader)
            .field("mix_buffer_len", &self.mix_buffer.len())
            .field("deck_node_count", &self.deck_node_ids.len())
            .finish()
    }
}

impl Mixer {
    /// Create a new mixer with a fixed number of deck slots (sources in the graph).
    pub fn new() -> Self {
        let max_decks = 2;
        let mut graph = MixerGraph::with_capacity(max_decks + 1, max_decks + 1);
        let mut deck_node_ids = Vec::with_capacity(max_decks);

        for _ in 0..max_decks {
            let id = graph.add_node(NodeData::new2(MixerNode::DeckSource(DeckSourceNode)));
            deck_node_ids.push(id);
        }
        let sum_node_id = graph.add_node(NodeData::new2(MixerNode::Sum(Sum)));

        for &deck_id in &deck_node_ids {
            graph.add_edge(deck_id, sum_node_id, ());
        }

        let processor = Processor::with_capacity(max_decks + 1);

        Self {
            master_volume: 1.0,
            cue_volume: 1.0,
            crossfader: 0.5,
            mix_buffer: Vec::new(),
            graph,
            processor,
            deck_node_ids,
            sum_node_id,
        }
    }

    /// Get the master volume
    pub fn master_volume(&self) -> f32 {
        self.master_volume
    }

    /// Set the master volume
    pub fn set_master_volume(&mut self, volume: f32) -> Result<()> {
        if !(0.0..=1.0).contains(&volume) {
            return Err(anyhow::anyhow!("Volume must be between 0.0 and 1.0"));
        }
        self.master_volume = volume;
        Ok(())
    }

    /// Get the cue volume
    pub fn cue_volume(&self) -> f32 {
        self.cue_volume
    }

    /// Set the cue volume
    pub fn set_cue_volume(&mut self, volume: f32) -> Result<()> {
        if !(0.0..=1.0).contains(&volume) {
            return Err(anyhow::anyhow!("Volume must be between 0.0 and 1.0"));
        }
        self.cue_volume = volume;
        Ok(())
    }

    /// Crossfader position (0.0 = deck A, 1.0 = deck B).
    pub fn crossfader(&self) -> f32 {
        self.crossfader
    }

    /// Set crossfader position (0.0 = deck A, 1.0 = deck B).
    pub fn set_crossfader(&mut self, position: f32) -> Result<()> {
        if !(0.0..=1.0).contains(&position) {
            return Err(anyhow::anyhow!("Crossfader must be between 0.0 and 1.0"));
        }
        self.crossfader = position;
        Ok(())
    }

    /// Equal-power crossfader gains for deck A and deck B.
    pub fn crossfader_gains(position: f32) -> (f32, f32) {
        let t = position.clamp(0.0, 1.0);
        let angle = t * std::f32::consts::FRAC_PI_2;
        (angle.cos(), angle.sin())
    }

    /// Process audio from all decks through the graph and route to output buses.
    pub fn process(
        &mut self,
        decks: &mut [Deck],
        frames: u32,
        output_buses: &mut HashMap<BusId, Vec<Sample>>,
    ) -> Result<()> {
        let buffer_size = frames as usize * 2;
        self.mix_buffer.resize(buffer_size, 0.0);
        self.mix_buffer.fill(0.0);

        // Collect deck outputs (interleaved stereo) so we can chunk without re-calling process.
        let mut deck_buffers: Vec<Vec<Sample>> = Vec::with_capacity(decks.len());
        for deck in decks.iter_mut() {
            let out = deck.process(frames)?;
            deck_buffers.push(out.to_vec());
        }

        let (gain_a, gain_b) = Self::crossfader_gains(self.crossfader);
        for (i, deck_buf) in deck_buffers.iter_mut().enumerate() {
            let gain = match i {
                0 => gain_a,
                1 => gain_b,
                _ => 1.0,
            };
            if gain != 1.0 {
                for sample in deck_buf.iter_mut() {
                    *sample *= gain;
                }
            }
        }

        let n_samples_per_channel = frames as usize;
        let n_chunks = n_samples_per_channel.div_ceil(CHUNK_SAMPLES);

        for chunk in 0..n_chunks {
            let start = chunk * CHUNK_SAMPLES;
            let len = (n_samples_per_channel - start).min(CHUNK_SAMPLES);

            // Fill each deck source node's buffers from deck output for this chunk.
            for (i, deck_buf) in deck_buffers.iter().enumerate() {
                if i >= self.deck_node_ids.len() {
                    break;
                }
                let node_id = self.deck_node_ids[i];
                let node_data = self.graph.node_weight_mut(node_id).unwrap();
                let buffers = &mut node_data.buffers;
                if buffers.len() >= 2 {
                    let (ch0, ch1) = buffers.split_at_mut(1);
                    let left = &mut ch0[0];
                    let right = &mut ch1[0];
                    for (s, (l, r)) in left[..len]
                        .iter_mut()
                        .zip(right[..len].iter_mut())
                        .enumerate()
                    {
                        let interleaved_idx = (start + s) * 2;
                        if interleaved_idx + 1 < deck_buf.len() {
                            *l = deck_buf[interleaved_idx];
                            *r = deck_buf[interleaved_idx + 1];
                        }
                    }
                    if len < CHUNK_SAMPLES {
                        left[len..CHUNK_SAMPLES].fill(0.0);
                        right[len..CHUNK_SAMPLES].fill(0.0);
                    }
                }
            }

            // Run the graph: sources -> Sum.
            process(&mut self.processor, &mut self.graph, self.sum_node_id);

            // Copy sum output into mix_buffer for this chunk.
            let sum_data = self.graph.node_weight(self.sum_node_id).unwrap();
            let out_start = chunk * CHUNK_SAMPLES * 2;
            for s in 0..len {
                if sum_data.buffers.len() >= 2 && out_start + s * 2 + 1 < self.mix_buffer.len() {
                    self.mix_buffer[out_start + s * 2] = sum_data.buffers[0][s];
                    self.mix_buffer[out_start + s * 2 + 1] = sum_data.buffers[1][s];
                }
            }
        }

        self.route_to_buses(frames, decks, output_buses)?;
        Ok(())
    }

    /// Route mixed audio to output buses and apply volume + clamp.
    fn route_to_buses(
        &self,
        frames: u32,
        decks: &[Deck],
        output_buses: &mut HashMap<BusId, Vec<Sample>>,
    ) -> Result<()> {
        let required_size = frames as usize * 2;

        for (bus_id, output_buffer) in output_buses.iter_mut() {
            if output_buffer.len() < required_size {
                output_buffer.resize(required_size, 0.0);
            }

            let bus_name = bus_id.as_str();
            match bus_name {
                "cue" => {
                    output_buffer.fill(0.0);
                    for deck in decks {
                        if deck.headphone_cue() && deck.state() == &DeckState::Playing {
                            let pre_fader = deck.pre_fader_buffer();
                            for (i, &sample) in pre_fader.iter().enumerate() {
                                if i < output_buffer.len() {
                                    output_buffer[i] += sample;
                                }
                            }
                        }
                    }
                }
                _ => {
                    for (i, &sample) in self.mix_buffer.iter().enumerate() {
                        if i < output_buffer.len() {
                            output_buffer[i] = sample;
                        }
                    }
                }
            }

            let volume = match bus_name {
                "cue" => self.cue_volume,
                "master" => self.master_volume,
                _ => 1.0,
            };

            if let Some(frames_slice) =
                audio_slice::to_frame_slice_mut::<&mut [Sample], StereoFrame>(output_buffer)
            {
                for frame in frames_slice.iter_mut() {
                    let scaled = frame.scale_amp(volume);
                    *frame = [scaled[0].clamp(-1.0, 1.0), scaled[1].clamp(-1.0, 1.0)];
                }
            } else {
                for sample in output_buffer.iter_mut() {
                    *sample = (*sample * volume).clamp(-1.0, 1.0);
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use audio_core::LoadedAudio;
    use std::sync::Arc;

    #[test]
    fn test_mixer_creation() {
        let mixer = Mixer::new();
        assert_eq!(mixer.master_volume(), 1.0);
        assert_eq!(mixer.cue_volume(), 1.0);
    }

    #[test]
    fn test_mixer_volume_controls() {
        let mut mixer = Mixer::new();
        mixer.set_master_volume(0.5).unwrap();
        assert_eq!(mixer.master_volume(), 0.5);
        mixer.set_cue_volume(0.7).unwrap();
        assert_eq!(mixer.cue_volume(), 0.7);
        assert!(mixer.set_master_volume(-0.1).is_err());
        assert!(mixer.set_master_volume(1.1).is_err());
    }

    fn load_test_tone(deck: &mut Deck) {
        // Longer than one mixer callback so multi-pass tests don't hit track-end.
        let audio = LoadedAudio {
            samples: vec![0.8f32; 4096 * 2],
            sample_rate: 48000,
            channels: 2,
            source_id: "test.wav".to_string(),
        };
        deck.load(Arc::new(audio)).unwrap();
    }

    #[test]
    fn test_mixer_processing() {
        let mut mixer = Mixer::new();
        let mut decks = vec![Deck::new(0, 48000, 512, "medium"), Deck::new(1, 48000, 512, "medium")];
        load_test_tone(&mut decks[0]);
        decks[0].play().unwrap();

        let mut output_buses = HashMap::new();
        output_buses.insert(BusId::new("master"), vec![0.0; 1024]);
        output_buses.insert(BusId::new("cue"), vec![0.0; 1024]);

        let result = mixer.process(&mut decks, 512, &mut output_buses);
        assert!(result.is_ok());

        let master_audio = &output_buses[&BusId::new("master")];
        let cue_audio = &output_buses[&BusId::new("cue")];
        assert!(master_audio.iter().any(|&s| s != 0.0));
        let cue_max = cue_audio
            .iter()
            .map(|&s| s.abs())
            .fold(0.0_f32, f32::max);
        assert!(cue_max < 1e-6, "cue should be silent without headphone cue");
    }

    #[test]
    fn cue_bus_silent_when_no_headphone_cue() {
        let mut mixer = Mixer::new();
        let mut decks = vec![Deck::new(0, 48000, 512, "medium")];
        load_test_tone(&mut decks[0]);
        decks[0].play().unwrap();
        // headphone_cue stays false

        let mut output_buses = HashMap::new();
        output_buses.insert(BusId::new("master"), vec![0.0; 1024]);
        output_buses.insert(BusId::new("cue"), vec![0.0; 1024]);
        mixer.process(&mut decks, 512, &mut output_buses).unwrap();

        let cue_max = output_buses[&BusId::new("cue")]
            .iter()
            .map(|&s| s.abs())
            .fold(0.0_f32, f32::max);
        assert!(cue_max < 1e-6, "cue should be silent, got {}", cue_max);
        assert!(output_buses[&BusId::new("master")]
            .iter()
            .any(|&s| s != 0.0));
    }

    #[test]
    fn cue_bus_sums_pre_fader_when_cued() {
        let mut mixer = Mixer::new();
        let mut decks = vec![Deck::new(0, 48000, 512, "medium")];
        load_test_tone(&mut decks[0]);
        decks[0].set_volume(0.0).unwrap(); // master silent
        decks[0].set_headphone_cue(true);
        decks[0].play().unwrap();

        let mut output_buses = HashMap::new();
        output_buses.insert(BusId::new("master"), vec![0.0; 1024]);
        output_buses.insert(BusId::new("cue"), vec![0.0; 1024]);
        mixer.process(&mut decks, 512, &mut output_buses).unwrap();

        let master_max = output_buses[&BusId::new("master")]
            .iter()
            .map(|&s| s.abs())
            .fold(0.0_f32, f32::max);
        let cue_max = output_buses[&BusId::new("cue")]
            .iter()
            .map(|&s| s.abs())
            .fold(0.0_f32, f32::max);
        assert!(master_max < 1e-6);
        assert!(cue_max > 0.1, "cued pre-fader should reach cue bus, got {}", cue_max);
    }

    #[test]
    fn test_mixer_volume_application() {
        let mut mixer = Mixer::new();
        mixer.set_master_volume(0.5).unwrap();
        mixer.set_cue_volume(0.3).unwrap();

        let mut decks = vec![Deck::new(0, 48000, 512, "medium")];
        load_test_tone(&mut decks[0]);
        decks[0].set_headphone_cue(true);
        decks[0].play().unwrap();

        let mut output_buses = HashMap::new();
        output_buses.insert(BusId::new("master"), vec![0.0; 1024]);
        output_buses.insert(BusId::new("cue"), vec![0.0; 1024]);

        mixer.process(&mut decks, 512, &mut output_buses).unwrap();

        let master_audio = &output_buses[&BusId::new("master")];
        let cue_audio = &output_buses[&BusId::new("cue")];
        let master_max = master_audio.iter().map(|&s| s.abs()).fold(0.0, f32::max);
        let cue_max = cue_audio.iter().map(|&s| s.abs()).fold(0.0, f32::max);
        assert!(master_max > 0.0);
        assert!(cue_max > 0.0);
        assert!(master_max > cue_max);
    }

    #[test]
    fn test_mixer_multiple_decks() {
        let mut mixer = Mixer::new();
        let mut decks = vec![Deck::new(0, 48000, 512, "medium"), Deck::new(1, 48000, 512, "medium")];
        load_test_tone(&mut decks[0]);
        load_test_tone(&mut decks[1]);
        decks[0].play().unwrap();
        decks[1].play().unwrap();

        let mut output_buses = HashMap::new();
        output_buses.insert(BusId::new("master"), vec![0.0; 1024]);

        mixer.process(&mut decks, 512, &mut output_buses).unwrap();

        let master_audio = &output_buses[&BusId::new("master")];
        assert!(master_audio.iter().any(|&s| s != 0.0));
    }

    #[test]
    fn test_crossfader_gains() {
        let (a, b) = Mixer::crossfader_gains(0.0);
        assert!((a - 1.0).abs() < 1e-6);
        assert!(b.abs() < 1e-6);

        let (a, b) = Mixer::crossfader_gains(1.0);
        assert!(a.abs() < 1e-6);
        assert!((b - 1.0).abs() < 1e-6);

        let (a, b) = Mixer::crossfader_gains(0.5);
        assert!((a - b).abs() < 1e-6);
    }

    #[test]
    fn test_crossfader_attenuates_deck() {
        let mut mixer = Mixer::new();
        mixer.set_crossfader(0.0).unwrap();

        let mut decks = vec![Deck::new(0, 48000, 512, "medium"), Deck::new(1, 48000, 512, "medium")];
        load_test_tone(&mut decks[0]);
        load_test_tone(&mut decks[1]);
        decks[0].play().unwrap();
        decks[1].play().unwrap();

        let mut output_buses = HashMap::new();
        output_buses.insert(BusId::new("master"), vec![0.0; 1024]);

        mixer.process(&mut decks, 512, &mut output_buses).unwrap();
        let full_a = output_buses[&BusId::new("master")]
            .iter()
            .map(|s| s.abs())
            .fold(0.0_f32, f32::max);

        decks[0].seek(0).unwrap();
        decks[1].seek(0).unwrap();
        // Seek does not resume playback after TrackEnded / pause.
        decks[0].play().unwrap();
        decks[1].play().unwrap();
        mixer.set_crossfader(1.0).unwrap();
        output_buses.insert(BusId::new("master"), vec![0.0; 1024]);
        mixer.process(&mut decks, 512, &mut output_buses).unwrap();
        let full_b = output_buses[&BusId::new("master")]
            .iter()
            .map(|s| s.abs())
            .fold(0.0_f32, f32::max);

        assert!(full_a > 0.0);
        assert!(full_b > 0.0);
        assert!((full_a - full_b).abs() < 0.2);
    }
}
