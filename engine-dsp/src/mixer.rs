//! Audio mixer for routing and mixing multiple lanes
//!
//! Each [`MixerLane`] is a graph node that owns its deck + strip. The mixer
//! renders lanes, applies crossfader gains when summing, then routes buses.

use anyhow::Result;
use audio_core::{slice as audio_slice, BusId, Frame, Sample, StereoFrame};
use dasp_graph::{process, Buffer, Input, Node, NodeData, Processor};
use petgraph::graph::DiGraph;
use std::collections::HashMap;

use crate::mixer_lane::MixerLane;
use crate::{HeadphoneMonitor, MixerChannel};

/// Fixed buffer length used by dasp_graph (samples per channel per process call).
const CHUNK_SAMPLES: usize = Buffer::LEN;

/// Node type in the mixer graph (one lane per deck slot).
#[derive(Debug)]
pub enum MixerNode {
    Lane(MixerLane),
}

impl Node for MixerNode {
    fn process(&mut self, inputs: &[Input], output: &mut [Buffer]) {
        match self {
            MixerNode::Lane(n) => n.process(inputs, output),
        }
    }
}

/// Graph type: directed graph with NodeData<MixerNode>, no edge weight.
type MixerGraph = DiGraph<NodeData<MixerNode>, (), u32>;

/// Audio mixer: process lanes, crossfade-sum, then route buses.
pub struct Mixer {
    master_volume: f32,
    cue_volume: f32,
    cue_mix: f32,
    master_cue: bool,
    /// Crossfader position: 0.0 = deck A, 1.0 = deck B.
    crossfader: f32,
    /// Internal mix buffer (interleaved stereo) filled from lane outputs.
    mix_buffer: Vec<Sample>,
    /// Reusable scratch for PFL sum before HeadphoneMonitor blend.
    pfl_scratch: Vec<Sample>,
    /// Graph: one lane node per slot.
    graph: MixerGraph,
    /// Processor for running lane nodes (reused to avoid allocs).
    processor: Processor<MixerGraph>,
    /// Node index for each lane.
    lane_node_ids: Vec<petgraph::graph::NodeIndex>,
}

impl std::fmt::Debug for Mixer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Mixer")
            .field("master_volume", &self.master_volume)
            .field("cue_volume", &self.cue_volume)
            .field("cue_mix", &self.cue_mix)
            .field("master_cue", &self.master_cue)
            .field("crossfader", &self.crossfader)
            .field("lane_count", &self.lane_node_ids.len())
            .field("mix_buffer_len", &self.mix_buffer.len())
            .finish()
    }
}

impl Mixer {
    /// Create a mixer with one lane graph node per slot.
    pub fn new(
        sample_rate: u32,
        buffer_size: u32,
        num_lanes: usize,
        resampler_quality: &str,
    ) -> Self {
        let buffer_size = buffer_size.max(1);
        let mut graph = MixerGraph::with_capacity(num_lanes, 0);
        let mut lane_node_ids = Vec::with_capacity(num_lanes);

        for i in 0..num_lanes {
            let lane = MixerLane::new(i, sample_rate, buffer_size, resampler_quality);
            let lane_id = graph.add_node(NodeData::new2(MixerNode::Lane(lane)));
            lane_node_ids.push(lane_id);
        }

        let processor = Processor::with_capacity(num_lanes.max(1));

        Self {
            master_volume: 1.0,
            cue_volume: 1.0,
            cue_mix: 0.0,
            master_cue: false,
            crossfader: 0.5,
            mix_buffer: Vec::new(),
            pfl_scratch: Vec::new(),
            graph,
            processor,
            lane_node_ids,
        }
    }

    /// Number of lanes (deck + channel pairs).
    pub fn num_lanes(&self) -> usize {
        self.lane_node_ids.len()
    }

    /// Get a mixer lane by index.
    pub fn lane(&self, index: usize) -> Option<&MixerLane> {
        let node_id = *self.lane_node_ids.get(index)?;
        match &self.graph.node_weight(node_id)?.node {
            MixerNode::Lane(lane) => Some(lane),
        }
    }

    /// Get a mutable mixer lane by index.
    pub fn lane_mut(&mut self, index: usize) -> Option<&mut MixerLane> {
        let node_id = *self.lane_node_ids.get(index)?;
        match &mut self.graph.node_weight_mut(node_id)?.node {
            MixerNode::Lane(lane) => Some(lane),
        }
    }

    /// Get a mixer channel by index (strip owned by the lane).
    pub fn channel(&self, index: usize) -> Option<&MixerChannel> {
        self.lane(index).map(|lane| lane.channel())
    }

    /// Get a mutable mixer channel by index.
    pub fn channel_mut(&mut self, index: usize) -> Option<&mut MixerChannel> {
        self.lane_mut(index).map(|lane| lane.channel_mut())
    }

    /// Set loudness-normalization target for all channels (`None` = off).
    pub fn set_normalizer_target(&mut self, target_lufs: Option<f32>) {
        for index in 0..self.lane_node_ids.len() {
            if let Some(channel) = self.channel_mut(index) {
                channel.set_target_lufs(target_lufs);
            }
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

    /// Cue blend: 0.0 = PFL only, 1.0 = master tap only (when `master_cue`).
    pub fn cue_mix(&self) -> f32 {
        self.cue_mix
    }

    /// Set cue blend (0.0 = PFL only, 1.0 = master tap only when `master_cue`).
    pub fn set_cue_mix(&mut self, mix: f32) -> Result<()> {
        if !(0.0..=1.0).contains(&mix) {
            return Err(anyhow::anyhow!("Cue mix must be between 0.0 and 1.0"));
        }
        self.cue_mix = mix;
        Ok(())
    }

    /// Whether the cue bus includes a pre-fader master tap.
    pub fn master_cue(&self) -> bool {
        self.master_cue
    }

    /// Enable or disable master tap on the cue bus.
    pub fn set_master_cue(&mut self, enabled: bool) {
        self.master_cue = enabled;
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

    /// Process audio from all lanes, apply crossfader when summing, and route buses.
    pub fn process(
        &mut self,
        frames: u32,
        output_buses: &mut HashMap<BusId, Vec<Sample>>,
    ) -> Result<()> {
        let buffer_size = frames as usize * 2;
        self.mix_buffer.resize(buffer_size, 0.0);
        self.mix_buffer.fill(0.0);

        for i in 0..self.lane_node_ids.len() {
            self.lane_mut(i)
                .expect("lane node index must identify a lane")
                .begin_render(frames)?;
        }

        let (gain_a, gain_b) = Self::crossfader_gains(self.crossfader);
        let n_samples_per_channel = frames as usize;
        let n_chunks = n_samples_per_channel.div_ceil(CHUNK_SAMPLES);

        for chunk in 0..n_chunks {
            let start = chunk * CHUNK_SAMPLES;
            let len = (n_samples_per_channel - start).min(CHUNK_SAMPLES);
            let out_start = chunk * CHUNK_SAMPLES * 2;

            for (i, &lane_id) in self.lane_node_ids.iter().enumerate() {
                process(&mut self.processor, &mut self.graph, lane_id);

                let gain = match i {
                    0 => gain_a,
                    1 => gain_b,
                    _ => 1.0,
                };
                let lane_data = self.graph.node_weight(lane_id).unwrap();
                if lane_data.buffers.len() < 2 {
                    continue;
                }
                for s in 0..len {
                    let out = out_start + s * 2;
                    if out + 1 < self.mix_buffer.len() {
                        self.mix_buffer[out] += lane_data.buffers[0][s] * gain;
                        self.mix_buffer[out + 1] += lane_data.buffers[1][s] * gain;
                    }
                }
            }
        }

        self.route_to_buses(frames, output_buses)?;
        Ok(())
    }

    /// Route mixed audio to output buses and apply volume + clamp.
    fn route_to_buses(
        &mut self,
        frames: u32,
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
                    self.pfl_scratch.resize(required_size, 0.0);
                    self.pfl_scratch.fill(0.0);
                    let graph = &self.graph;
                    let pfl_scratch = &mut self.pfl_scratch;
                    for &node_id in &self.lane_node_ids {
                        let lane = match &graph
                            .node_weight(node_id)
                            .expect("lane node index must be valid")
                            .node
                        {
                            MixerNode::Lane(lane) => lane,
                        };
                        let channel = lane.channel();
                        if channel.headphone_cue() {
                            let pre_fader = channel.pre_fader_buffer();
                            for (i, &sample) in pre_fader.iter().enumerate() {
                                if i < pfl_scratch.len() {
                                    pfl_scratch[i] += sample;
                                }
                            }
                        }
                    }
                    output_buffer.fill(0.0);
                    HeadphoneMonitor::render(
                        &self.pfl_scratch,
                        &self.mix_buffer,
                        self.cue_mix,
                        self.master_cue,
                        output_buffer,
                    );
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

    fn new_mixer(num_lanes: usize) -> Mixer {
        Mixer::new(48_000, 512, num_lanes, "medium")
    }

    fn load_test_tone(mixer: &mut Mixer, lane: usize) {
        let audio = LoadedAudio {
            samples: vec![0.8f32; 4096 * 2],
            sample_rate: 48000,
            channels: 2,
            source_id: "test.wav".to_string(),
        };
        mixer
            .lane_mut(lane)
            .unwrap()
            .deck_mut()
            .load(Arc::new(audio))
            .unwrap();
    }

    #[test]
    fn test_mixer_creation() {
        let mixer = new_mixer(2);
        assert_eq!(mixer.master_volume(), 1.0);
        assert_eq!(mixer.cue_volume(), 1.0);
        assert_eq!(mixer.num_lanes(), 2);
        assert!(mixer.lane(0).is_some());
        assert!(mixer.channel(0).is_some());
        assert!(mixer.channel(1).is_some());
        assert!(mixer.channel(2).is_none());
        assert!(mixer.lane(2).is_none());
    }

    #[test]
    fn test_mixer_volume_controls() {
        let mut mixer = new_mixer(2);
        mixer.set_master_volume(0.5).unwrap();
        assert_eq!(mixer.master_volume(), 0.5);
        mixer.set_cue_volume(0.7).unwrap();
        assert_eq!(mixer.cue_volume(), 0.7);
        assert!(mixer.set_master_volume(-0.1).is_err());
        assert!(mixer.set_master_volume(1.1).is_err());
    }

    #[test]
    fn test_mixer_processing() {
        let mut mixer = new_mixer(2);
        load_test_tone(&mut mixer, 0);
        mixer.lane_mut(0).unwrap().deck_mut().play().unwrap();

        let mut output_buses = HashMap::new();
        output_buses.insert(BusId::new("master"), vec![0.0; 1024]);
        output_buses.insert(BusId::new("cue"), vec![0.0; 1024]);

        let result = mixer.process(512, &mut output_buses);
        assert!(result.is_ok());

        let master_audio = &output_buses[&BusId::new("master")];
        let cue_audio = &output_buses[&BusId::new("cue")];
        assert!(master_audio.iter().any(|&s| s != 0.0));
        let cue_max = cue_audio.iter().map(|&s| s.abs()).fold(0.0_f32, f32::max);
        assert!(cue_max < 1e-6, "cue should be silent without headphone cue");
    }

    #[test]
    fn cue_mix_and_master_cue_defaults() {
        let mixer = new_mixer(2);
        assert_eq!(mixer.cue_mix(), 0.0);
        assert!(!mixer.master_cue());
    }

    #[test]
    fn set_cue_mix_rejects_out_of_range() {
        let mut mixer = new_mixer(2);
        assert!(mixer.set_cue_mix(-0.1).is_err());
        assert!(mixer.set_cue_mix(1.1).is_err());
        mixer.set_cue_mix(0.5).unwrap();
        assert_eq!(mixer.cue_mix(), 0.5);
    }

    #[test]
    fn master_cue_on_mix_one_hears_master_without_pfl() {
        let mut mixer = new_mixer(1);
        mixer.set_master_cue(true);
        mixer.set_cue_mix(1.0).unwrap();
        mixer.set_master_volume(0.25).unwrap();
        mixer.set_cue_volume(1.0).unwrap();

        load_test_tone(&mut mixer, 0);
        mixer.lane_mut(0).unwrap().deck_mut().play().unwrap();

        let mut output_buses = HashMap::new();
        output_buses.insert(BusId::new("master"), vec![0.0; 1024]);
        output_buses.insert(BusId::new("cue"), vec![0.0; 1024]);
        mixer.process(512, &mut output_buses).unwrap();

        let cue_max = output_buses[&BusId::new("cue")]
            .iter()
            .map(|&s| s.abs())
            .fold(0.0_f32, f32::max);
        let master_max = output_buses[&BusId::new("master")]
            .iter()
            .map(|&s| s.abs())
            .fold(0.0_f32, f32::max);
        assert!(
            cue_max > 0.1,
            "cue should carry master tap, got {}",
            cue_max
        );
        assert!(
            cue_max > master_max + 0.05,
            "cue tap must be pre master_volume (cue {}, master {})",
            cue_max,
            master_max
        );
    }

    #[test]
    fn master_cue_off_mix_one_stays_silent_without_pfl() {
        let mut mixer = new_mixer(1);
        mixer.set_master_cue(false);
        mixer.set_cue_mix(1.0).unwrap();

        load_test_tone(&mut mixer, 0);
        mixer.lane_mut(0).unwrap().deck_mut().play().unwrap();

        let mut output_buses = HashMap::new();
        output_buses.insert(BusId::new("master"), vec![0.0; 1024]);
        output_buses.insert(BusId::new("cue"), vec![0.0; 1024]);
        mixer.process(512, &mut output_buses).unwrap();

        let cue_max = output_buses[&BusId::new("cue")]
            .iter()
            .map(|&s| s.abs())
            .fold(0.0_f32, f32::max);
        assert!(
            cue_max < 1e-6,
            "no master bleed when Master Cue off, got {}",
            cue_max
        );
    }

    #[test]
    fn cue_bus_silent_when_no_headphone_cue() {
        let mut mixer = new_mixer(1);
        load_test_tone(&mut mixer, 0);
        mixer.lane_mut(0).unwrap().deck_mut().play().unwrap();

        let mut output_buses = HashMap::new();
        output_buses.insert(BusId::new("master"), vec![0.0; 1024]);
        output_buses.insert(BusId::new("cue"), vec![0.0; 1024]);
        mixer.process(512, &mut output_buses).unwrap();

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
        let mut mixer = new_mixer(1);
        load_test_tone(&mut mixer, 0);
        mixer.channel_mut(0).unwrap().set_volume(0.0).unwrap();
        mixer.channel_mut(0).unwrap().set_headphone_cue(true);
        mixer.lane_mut(0).unwrap().deck_mut().play().unwrap();

        let mut output_buses = HashMap::new();
        output_buses.insert(BusId::new("master"), vec![0.0; 1024]);
        output_buses.insert(BusId::new("cue"), vec![0.0; 1024]);
        mixer.process(512, &mut output_buses).unwrap();

        let master_max = output_buses[&BusId::new("master")]
            .iter()
            .map(|&s| s.abs())
            .fold(0.0_f32, f32::max);
        let cue_max = output_buses[&BusId::new("cue")]
            .iter()
            .map(|&s| s.abs())
            .fold(0.0_f32, f32::max);
        assert!(master_max < 1e-6);
        assert!(
            cue_max > 0.1,
            "cued pre-fader should reach cue bus, got {}",
            cue_max
        );
    }

    #[test]
    fn test_mixer_volume_application() {
        let mut mixer = new_mixer(1);
        mixer.set_master_volume(0.5).unwrap();
        mixer.set_cue_volume(0.3).unwrap();

        load_test_tone(&mut mixer, 0);
        mixer.channel_mut(0).unwrap().set_headphone_cue(true);
        mixer.lane_mut(0).unwrap().deck_mut().play().unwrap();

        let mut output_buses = HashMap::new();
        output_buses.insert(BusId::new("master"), vec![0.0; 1024]);
        output_buses.insert(BusId::new("cue"), vec![0.0; 1024]);

        mixer.process(512, &mut output_buses).unwrap();

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
        let mut mixer = new_mixer(2);
        load_test_tone(&mut mixer, 0);
        load_test_tone(&mut mixer, 1);
        mixer.lane_mut(0).unwrap().deck_mut().play().unwrap();
        mixer.lane_mut(1).unwrap().deck_mut().play().unwrap();

        let mut output_buses = HashMap::new();
        output_buses.insert(BusId::new("master"), vec![0.0; 1024]);

        mixer.process(512, &mut output_buses).unwrap();

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
        let mut mixer = new_mixer(2);
        mixer.set_crossfader(0.0).unwrap();

        load_test_tone(&mut mixer, 0);
        load_test_tone(&mut mixer, 1);
        mixer.lane_mut(0).unwrap().deck_mut().play().unwrap();
        mixer.lane_mut(1).unwrap().deck_mut().play().unwrap();

        let mut output_buses = HashMap::new();
        output_buses.insert(BusId::new("master"), vec![0.0; 1024]);

        mixer.process(512, &mut output_buses).unwrap();
        let full_a = output_buses[&BusId::new("master")]
            .iter()
            .map(|s| s.abs())
            .fold(0.0_f32, f32::max);

        mixer.lane_mut(0).unwrap().deck_mut().seek(0).unwrap();
        mixer.lane_mut(1).unwrap().deck_mut().seek(0).unwrap();
        mixer.lane_mut(0).unwrap().deck_mut().play().unwrap();
        mixer.lane_mut(1).unwrap().deck_mut().play().unwrap();
        mixer.set_crossfader(1.0).unwrap();
        output_buses.insert(BusId::new("master"), vec![0.0; 1024]);
        mixer.process(512, &mut output_buses).unwrap();
        let full_b = output_buses[&BusId::new("master")]
            .iter()
            .map(|s| s.abs())
            .fold(0.0_f32, f32::max);

        assert!(full_a > 0.0);
        assert!(full_b > 0.0);
        assert!((full_a - full_b).abs() < 0.2);
    }
}
