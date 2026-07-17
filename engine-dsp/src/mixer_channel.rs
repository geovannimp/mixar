//! A complete stereo channel strip (gain, EQ, filter, VU/PFL, fader).

use anyhow::Result;
use audio_core::Sample;
use dasp_graph::{Buffer, Input, Node};

use crate::eq::{clamp_gain_db, DeckEqGains, ThreeBandEq};
use crate::filter::{db_to_linear, DjFilter};
use crate::level_meter::LevelPeaks;

/// Per-lane mixer strip state and DSP.
///
/// Owned by [`crate::MixerLane`]. Implements [`Node`] for focused strip tests;
/// the live mixer feeds dry deck PCM via [`Self::process_dry_chunk`].
#[derive(Debug)]
pub struct MixerChannel {
    gain_trim_db: f32,
    auto_gain_db: f32,
    eq: ThreeBandEq,
    filter: DjFilter,
    volume: f32,
    headphone_cue: bool,
    level_peaks: LevelPeaks,
    pre_fader_buffer: Vec<Sample>,
    crossfader_gain: f32,
    active_sample_count: usize,
    processed_sample_count: usize,
}

impl MixerChannel {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            gain_trim_db: 0.0,
            auto_gain_db: 0.0,
            eq: ThreeBandEq::new(sample_rate),
            filter: DjFilter::new(sample_rate),
            volume: 1.0,
            headphone_cue: false,
            level_peaks: LevelPeaks::default(),
            pre_fader_buffer: Vec::new(),
            crossfader_gain: 1.0,
            active_sample_count: 0,
            processed_sample_count: 0,
        }
    }

    pub fn volume(&self) -> f32 {
        self.volume
    }

    pub fn set_volume(&mut self, volume: f32) -> Result<()> {
        if !(0.0..=1.0).contains(&volume) {
            return Err(anyhow::anyhow!("Volume must be between 0.0 and 1.0"));
        }
        self.volume = volume;
        Ok(())
    }

    pub fn eq_gains(&self) -> DeckEqGains {
        self.eq.gains()
    }

    pub fn set_eq_gains(&mut self, gains: DeckEqGains) -> Result<()> {
        self.eq.set_gains(gains)
    }

    pub fn set_eq_low_db(&mut self, gain_db: f32) -> Result<()> {
        self.eq.set_low_db(gain_db)
    }

    pub fn set_eq_mid_db(&mut self, gain_db: f32) -> Result<()> {
        self.eq.set_mid_db(gain_db)
    }

    pub fn set_eq_high_db(&mut self, gain_db: f32) -> Result<()> {
        self.eq.set_high_db(gain_db)
    }

    pub fn filter_db(&self) -> f32 {
        self.filter.filter_db()
    }

    pub fn set_filter_db(&mut self, filter_db: f32) -> Result<()> {
        self.filter.set_filter_db(filter_db);
        Ok(())
    }

    pub fn gain_trim_db(&self) -> f32 {
        self.gain_trim_db
    }

    pub fn set_gain_trim_db(&mut self, gain_db: f32) -> Result<()> {
        self.gain_trim_db = clamp_gain_db(gain_db);
        Ok(())
    }

    pub fn auto_gain_db(&self) -> f32 {
        self.auto_gain_db
    }

    pub fn set_auto_gain_db(&mut self, gain_db: f32) -> Result<()> {
        self.auto_gain_db = clamp_gain_db(gain_db);
        Ok(())
    }

    pub fn headphone_cue(&self) -> bool {
        self.headphone_cue
    }

    pub fn set_headphone_cue(&mut self, enabled: bool) {
        self.headphone_cue = enabled;
    }

    pub fn level_peaks(&self) -> LevelPeaks {
        self.level_peaks
    }

    pub fn pre_fader_buffer(&self) -> &[Sample] {
        &self.pre_fader_buffer
    }

    pub fn crossfader_gain(&self) -> f32 {
        self.crossfader_gain
    }

    pub fn set_crossfader_gain(&mut self, gain: f32) -> Result<()> {
        if !(0.0..=1.0).contains(&gain) {
            return Err(anyhow::anyhow!(
                "Crossfader gain must be between 0.0 and 1.0"
            ));
        }
        self.crossfader_gain = gain;
        Ok(())
    }

    /// Reset per-render capture state and prepare storage for interleaved samples.
    pub fn begin_render(&mut self, sample_count: usize) {
        self.pre_fader_buffer.clear();
        self.pre_fader_buffer.reserve(sample_count);
        self.level_peaks = LevelPeaks::default();
        self.active_sample_count = sample_count;
        self.processed_sample_count = 0;
    }

    /// Process the next graph chunk from interleaved dry PCM into stereo outputs.
    pub fn process_dry_chunk(&mut self, dry: &[Sample], output: &mut [Buffer]) {
        let dry_base = self.processed_sample_count;
        self.process_with_stereo_input(output, |frame| {
            let idx = dry_base + frame * 2;
            if idx + 1 < dry.len() {
                (dry[idx], dry[idx + 1])
            } else {
                (0.0, 0.0)
            }
        });
    }

    fn process_with_stereo_input(
        &mut self,
        output: &mut [Buffer],
        mut input_at: impl FnMut(usize) -> (f32, f32),
    ) {
        for buffer in output.iter_mut().skip(2) {
            buffer.silence();
        }
        if output.len() < 2 {
            for buffer in output {
                buffer.silence();
            }
            return;
        }

        let (left_output, right_output) = output.split_at_mut(1);
        let remaining_samples = self
            .active_sample_count
            .saturating_sub(self.processed_sample_count);
        if remaining_samples == 0 {
            left_output[0].silence();
            right_output[0].silence();
            return;
        }

        let chunk_frames = (remaining_samples / 2).min(Buffer::LEN);
        let chunk_samples = chunk_frames * 2;
        let chunk_start = self.pre_fader_buffer.len();
        self.pre_fader_buffer
            .resize(chunk_start + chunk_samples, 0.0);
        let gain = db_to_linear(self.auto_gain_db + self.gain_trim_db);

        for frame in 0..chunk_frames {
            let (left, right) = input_at(frame);
            self.pre_fader_buffer[chunk_start + frame * 2] = left * gain;
            self.pre_fader_buffer[chunk_start + frame * 2 + 1] = right * gain;
        }

        let chunk = &mut self.pre_fader_buffer[chunk_start..];
        self.eq.process_buffer(chunk);
        self.filter.process_buffer(chunk);

        let chunk_peaks = LevelPeaks::from_buffer(chunk);
        self.level_peaks.peak_l = self.level_peaks.peak_l.max(chunk_peaks.peak_l);
        self.level_peaks.peak_r = self.level_peaks.peak_r.max(chunk_peaks.peak_r);

        let post_fader_gain = self.volume * self.crossfader_gain;
        for frame in 0..chunk_frames {
            left_output[0][frame] = chunk[frame * 2] * post_fader_gain;
            right_output[0][frame] = chunk[frame * 2 + 1] * post_fader_gain;
        }
        for frame in chunk_frames..Buffer::LEN {
            left_output[0][frame] = 0.0;
            right_output[0][frame] = 0.0;
        }

        self.processed_sample_count += chunk_samples;
    }
}

impl Node for MixerChannel {
    fn process(&mut self, inputs: &[Input], output: &mut [Buffer]) {
        self.process_with_stereo_input(output, |frame| {
            let mut left = 0.0;
            let mut right = 0.0;
            for input in inputs {
                let buffers = input.buffers();
                if buffers.len() >= 2 {
                    left += buffers[0][frame];
                    right += buffers[1][frame];
                }
            }
            (left, right)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::MixerChannel;
    use crate::DeckEqGains;
    use dasp_graph::{process, Buffer, Input, Node, NodeData, Processor};
    use petgraph::graph::{DiGraph, NodeIndex};

    #[derive(Debug)]
    enum TestNode {
        Source,
        Channel(MixerChannel),
    }

    impl Node for TestNode {
        fn process(&mut self, inputs: &[Input], output: &mut [Buffer]) {
            match self {
                Self::Source => {}
                Self::Channel(channel) => channel.process(inputs, output),
            }
        }
    }

    type TestGraph = DiGraph<NodeData<TestNode>, (), u32>;

    fn test_graph(
        channel: MixerChannel,
        left: f32,
        right: f32,
    ) -> (TestGraph, NodeIndex, NodeIndex, Processor<TestGraph>) {
        let mut graph = TestGraph::with_capacity(2, 1);
        let source_id = graph.add_node(NodeData::new2(TestNode::Source));
        let channel_id = graph.add_node(NodeData::new2(TestNode::Channel(channel)));
        graph.add_edge(source_id, channel_id, ());
        graph.node_weight_mut(source_id).unwrap().buffers[0].fill(left);
        graph.node_weight_mut(source_id).unwrap().buffers[1].fill(right);
        let processor = Processor::with_capacity(2);
        (graph, source_id, channel_id, processor)
    }

    fn channel(graph: &TestGraph, channel_id: NodeIndex) -> &MixerChannel {
        match &graph.node_weight(channel_id).unwrap().node {
            TestNode::Channel(channel) => channel,
            TestNode::Source => panic!("expected channel node"),
        }
    }

    fn channel_mut(graph: &mut TestGraph, channel_id: NodeIndex) -> &mut MixerChannel {
        match &mut graph.node_weight_mut(channel_id).unwrap().node {
            TestNode::Channel(channel) => channel,
            TestNode::Source => panic!("expected channel node"),
        }
    }

    #[test]
    fn mixer_channel_defaults_and_control_validation() {
        let mut channel = MixerChannel::new(48_000);

        assert_eq!(channel.auto_gain_db(), 0.0);
        assert_eq!(channel.gain_trim_db(), 0.0);
        assert_eq!(channel.volume(), 1.0);
        assert_eq!(channel.crossfader_gain(), 1.0);
        assert_eq!(channel.eq_gains(), DeckEqGains::default());
        assert_eq!(channel.filter_db(), 0.0);
        assert!(!channel.headphone_cue());
        assert_eq!(channel.pre_fader_buffer(), &[]);
        assert_eq!(channel.level_peaks().peak_l, 0.0);
        assert_eq!(channel.level_peaks().peak_r, 0.0);

        assert!(channel.set_volume(1.1).is_err());
        assert!(channel.set_crossfader_gain(-0.1).is_err());
        channel.set_eq_low_db(6.0).unwrap();
        channel.set_eq_mid_db(-3.0).unwrap();
        channel.set_eq_high_db(2.0).unwrap();
        channel.set_filter_db(4.0).unwrap();
        channel.set_gain_trim_db(3.0).unwrap();
        channel.set_auto_gain_db(6.0).unwrap();
        channel.set_headphone_cue(true);

        assert_eq!(channel.eq_gains(), DeckEqGains::clamped(6.0, -3.0, 2.0));
        assert_eq!(channel.filter_db(), 4.0);
        assert_eq!(channel.gain_trim_db(), 3.0);
        assert_eq!(channel.auto_gain_db(), 6.0);
        assert!(channel.headphone_cue());
    }

    #[test]
    fn auto_gain_raises_pre_fader_output() {
        let mut baseline = MixerChannel::new(48_000);
        baseline.begin_render(Buffer::LEN * 2);
        let (mut graph, _, channel_id, mut processor) = test_graph(baseline, 0.25, -0.25);
        process(&mut processor, &mut graph, channel_id);
        let baseline_peak = channel(&graph, channel_id).level_peaks().peak_l;

        let mut boosted = MixerChannel::new(48_000);
        boosted.set_auto_gain_db(6.0).unwrap();
        boosted.begin_render(Buffer::LEN * 2);
        let (mut graph, _, channel_id, mut processor) = test_graph(boosted, 0.25, -0.25);
        process(&mut processor, &mut graph, channel_id);
        let boosted_peak = channel(&graph, channel_id).level_peaks().peak_l;

        let ratio = boosted_peak / baseline_peak;
        assert!(
            (ratio - 2.0).abs() < 0.15,
            "+6 dB should roughly double the pre-fader peak, got {ratio}"
        );
    }

    #[test]
    fn closed_fader_silences_output_but_preserves_pfl_and_peaks() {
        let mut mixer_channel = MixerChannel::new(48_000);
        mixer_channel.set_volume(0.0).unwrap();
        mixer_channel.begin_render(Buffer::LEN * 2);
        let (mut graph, _, channel_id, mut processor) = test_graph(mixer_channel, 0.5, -0.25);

        process(&mut processor, &mut graph, channel_id);

        let data = graph.node_weight(channel_id).unwrap();
        assert!(data
            .buffers
            .iter()
            .all(|buffer| buffer[..].iter().all(|sample| *sample == 0.0)));
        let mixer_channel = channel(&graph, channel_id);
        assert!(mixer_channel
            .pre_fader_buffer()
            .iter()
            .any(|sample| *sample != 0.0));
        assert!((mixer_channel.level_peaks().peak_l - 0.5).abs() < 1e-6);
        assert!((mixer_channel.level_peaks().peak_r - 0.25).abs() < 1e-6);
    }

    #[test]
    fn process_limits_active_frames_below_buffer_len() {
        const ACTIVE_FRAMES: usize = 16;
        let active_samples = ACTIVE_FRAMES * 2;

        let mut mixer_channel = MixerChannel::new(48_000);
        mixer_channel.begin_render(active_samples);
        let (mut graph, _, channel_id, mut processor) = test_graph(mixer_channel, 0.5, 0.25);

        process(&mut processor, &mut graph, channel_id);

        let mixer_channel = channel(&graph, channel_id);
        assert_eq!(
            mixer_channel.pre_fader_buffer().len(),
            active_samples,
            "PFL should contain only active interleaved samples"
        );

        let data = graph.node_weight(channel_id).unwrap();
        for frame in 0..ACTIVE_FRAMES {
            assert!(
                (data.buffers[0][frame] - 0.5).abs() < 1e-6,
                "active left frame {frame} should be processed"
            );
            assert!(
                (data.buffers[1][frame] - 0.25).abs() < 1e-6,
                "active right frame {frame} should be processed"
            );
        }
        for frame in ACTIVE_FRAMES..Buffer::LEN {
            assert_eq!(
                data.buffers[0][frame], 0.0,
                "inactive left frame {frame} should be silent"
            );
            assert_eq!(
                data.buffers[1][frame], 0.0,
                "inactive right frame {frame} should be silent"
            );
        }
    }

    #[test]
    fn process_limits_final_partial_chunk_across_multiple_calls() {
        const ACTIVE_FRAMES: usize = Buffer::LEN + 16;
        let active_samples = ACTIVE_FRAMES * 2;

        let mut mixer_channel = MixerChannel::new(48_000);
        mixer_channel.begin_render(active_samples);
        let (mut graph, source_id, channel_id, mut processor) =
            test_graph(mixer_channel, 0.5, 0.25);

        process(&mut processor, &mut graph, channel_id);
        graph.node_weight_mut(source_id).unwrap().buffers[0].fill(0.2);
        graph.node_weight_mut(source_id).unwrap().buffers[1].fill(0.1);
        process(&mut processor, &mut graph, channel_id);

        let mixer_channel = channel(&graph, channel_id);
        assert_eq!(
            mixer_channel.pre_fader_buffer().len(),
            active_samples,
            "PFL should stop at the active sample count"
        );
        assert!((mixer_channel.level_peaks().peak_l - 0.5).abs() < 1e-6);
        assert!((mixer_channel.level_peaks().peak_r - 0.25).abs() < 1e-6);

        let data = graph.node_weight(channel_id).unwrap();
        for frame in 16..Buffer::LEN {
            assert_eq!(
                data.buffers[0][frame], 0.0,
                "final chunk inactive left frame {frame} should be silent"
            );
            assert_eq!(
                data.buffers[1][frame], 0.0,
                "final chunk inactive right frame {frame} should be silent"
            );
        }
    }

    #[test]
    fn begin_render_resets_capture_and_process_appends_each_chunk() {
        let mut mixer_channel = MixerChannel::new(48_000);
        mixer_channel.begin_render(Buffer::LEN * 4);
        let (mut graph, source_id, channel_id, mut processor) =
            test_graph(mixer_channel, 0.5, 0.25);

        process(&mut processor, &mut graph, channel_id);
        graph.node_weight_mut(source_id).unwrap().buffers[0].fill(0.2);
        graph.node_weight_mut(source_id).unwrap().buffers[1].fill(0.1);
        process(&mut processor, &mut graph, channel_id);

        let mixer_channel = channel(&graph, channel_id);
        assert_eq!(mixer_channel.pre_fader_buffer().len(), Buffer::LEN * 4);
        assert!((mixer_channel.level_peaks().peak_l - 0.5).abs() < 1e-6);
        assert!((mixer_channel.level_peaks().peak_r - 0.25).abs() < 1e-6);

        channel_mut(&mut graph, channel_id).begin_render(Buffer::LEN * 2);
        let mixer_channel = channel(&graph, channel_id);
        assert!(mixer_channel.pre_fader_buffer().is_empty());
        assert_eq!(mixer_channel.level_peaks().peak_l, 0.0);
        assert_eq!(mixer_channel.level_peaks().peak_r, 0.0);
    }
}
