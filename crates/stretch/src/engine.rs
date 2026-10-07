//! timestretch realtime engine wrapper (WideKeylock = tempo without pitch).
//!
//! A realtime pitch factor is layered on top: the keylock stage runs at
//! `tempo / pitch` (holding pitch) and a stateful output [`StreamingSincResampler`]
//! restores the requested tempo while shifting pitch by `pitch`. At
//! `pitch == 1.0` the resampler is bypassed so output is bit-identical to the
//! plain keylock path.

use std::sync::Arc;

use crate::{StretchPullStats, TimeStretcher};
use anyhow::{anyhow, Result};
use audio_core::Sample;
use timestretch::core::resample::{SincInterpTable, StreamingSincResampler};
use timestretch::engine::{
    Engine, EngineConfig, EngineController, EngineProcessor, EngineProfile, SourceProducer,
    MAX_TEMPO_RATE,
};

/// Pitch factors within this of `1.0` bypass the resampler entirely.
const PITCH_BYPASS_EPSILON: f64 = 1e-6;
/// Pitch factor clamp range (matches the controller's tempo range).
const MIN_PITCH_FACTOR: f64 = 0.25;
const MAX_PITCH_FACTOR: f64 = 4.0;
/// Compact `carry` once the drained prefix passes this many samples.
const CARRY_COMPACT_THRESHOLD: usize = 4096;

/// timestretch WideKeylock stretcher for DJ key lock.
pub struct TimestretchStretcher {
    sample_rate: u32,
    controller: EngineController,
    processor: EngineProcessor,
    source: SourceProducer,
    feed_scratch: Vec<Sample>,
    /// Requested tempo rate (what the caller asked for; pitch-independent).
    tempo_rate: f64,
    /// Pitch multiplier (`1.0` = original).
    pitch: f64,
    /// Largest engine block per `processor.process` call.
    engine_chunk: usize,
    resampler_l: StreamingSincResampler,
    resampler_r: StreamingSincResampler,
    /// Engine-rate interleaved output before resampling.
    scratch: Vec<Sample>,
    /// Deinterleaved engine-rate channels feeding the resamplers.
    scratch_l: Vec<f32>,
    scratch_r: Vec<f32>,
    /// Resampled (pitched) mono channels awaiting interleave.
    resampled_l: Vec<f32>,
    resampled_r: Vec<f32>,
    /// Interleaved pitched frames buffered for the caller.
    carry: Vec<Sample>,
    /// Drained prefix length of `carry`, in samples.
    carry_head: usize,
}

impl TimestretchStretcher {
    /// Create a WideKeylock engine at `sample_rate` (stereo).
    pub fn new(sample_rate: u32, max_process_frames: usize) -> Result<Self> {
        if sample_rate == 0 {
            return Err(anyhow!("sample_rate must be > 0"));
        }
        let max_block = max_process_frames.clamp(64, 8192);
        // Cover ≥4× fastest-tempo callbacks (EngineConfig::validate).
        let min_source = ((max_block as f64 * MAX_TEMPO_RATE).ceil() as usize).saturating_mul(4);
        let source_capacity_frames = min_source.max(32_768);

        let handles = Engine::build(EngineConfig {
            sample_rate,
            channels: 2,
            profile: EngineProfile::WideKeylock,
            initial_tempo_rate: 1.0,
            max_block_frames: max_block,
            source_capacity_frames,
            pre_analysis: None,
        })
        .map_err(|e| anyhow!("timestretch Engine::build: {e}"))?;

        // Keylock default is on; keep it explicit for the deck path.
        handles.controller.set_keylock(true);

        // One shared windowed-sinc prototype for both channels.
        let table = SincInterpTable::new_stream_default();

        Ok(Self {
            sample_rate,
            controller: handles.controller,
            processor: handles.processor,
            source: handles.source,
            feed_scratch: vec![0.0; max_block.max(512) * 2],
            tempo_rate: 1.0,
            pitch: 1.0,
            engine_chunk: max_block,
            resampler_l: StreamingSincResampler::new(Arc::clone(&table)),
            resampler_r: StreamingSincResampler::new(table),
            scratch: Vec::new(),
            scratch_l: Vec::new(),
            scratch_r: Vec::new(),
            resampled_l: Vec::new(),
            resampled_r: Vec::new(),
            carry: Vec::new(),
            carry_head: 0,
        })
    }

    /// Whether the pitch resampler is active (not a unity bypass).
    fn pitch_engaged(&self) -> bool {
        (self.pitch - 1.0).abs() >= PITCH_BYPASS_EPSILON
    }

    /// Tempo rate handed to the keylock engine: `tempo / pitch` when pitch is
    /// engaged (the downstream resampler restores the requested tempo).
    fn engine_tempo(&self) -> f64 {
        if self.pitch_engaged() {
            self.tempo_rate / self.pitch
        } else {
            self.tempo_rate
        }
    }

    fn top_up(
        &mut self,
        out_frames: usize,
        feed: &mut dyn FnMut(usize, &mut [Sample]) -> usize,
    ) -> usize {
        let demand = self
            .source
            .demand_hint(out_frames, self.engine_tempo().max(1.0));
        let mut need = demand.saturating_sub(self.source.occupied_frames());
        let mut source_fed = 0usize;

        while need > 0 {
            let free = self.source.free_frames();
            if free == 0 {
                break;
            }
            let chunk = need.min(free).min(self.feed_scratch.len() / 2).max(1);
            if chunk * 2 > self.feed_scratch.len() {
                self.feed_scratch.resize(chunk * 2, 0.0);
            }
            let scratch = &mut self.feed_scratch[..chunk * 2];
            scratch.fill(0.0);
            let got = feed(chunk, scratch);
            source_fed += got;
            // Push whole chunk (silence-padded if feed short) so the ring stays paced.
            let accepted = self.source.push(scratch);
            if accepted == 0 {
                break;
            }
            need = need.saturating_sub(accepted);
            if got == 0 && accepted < chunk {
                break;
            }
        }
        source_fed
    }

    /// Pitch-aware pull: drives the keylock engine at `tempo / pitch` and runs
    /// its output through the streaming sinc resampler at `pitch`, buffering
    /// however much is needed to satisfy exactly `out_frames`.
    fn pull_pitched(
        &mut self,
        out_frames: usize,
        output: &mut [Sample],
        feed: &mut dyn FnMut(usize, &mut [Sample]) -> usize,
    ) -> StretchPullStats {
        let need_samples = out_frames * 2;
        let pitch = self.pitch;
        let mut source_frames_fed = 0usize;

        loop {
            let carried = (self.carry.len() - self.carry_head) / 2;
            if carried >= out_frames {
                break;
            }
            let need = out_frames - carried;
            let half_span = self.resampler_l.current_half_span();
            // The lookahead needs a floor so the resampler can always emit at
            // least one frame; `engine_chunk` bounds the per-call engine work
            // to what the source ring can actually hold.
            let floor = half_span + 8;
            let target = ((need as f64) * pitch).ceil() as usize + floor;
            let engine_frames = target.min(self.engine_chunk.max(floor));

            let carried_before = carried;
            let fed = self.top_up(engine_frames, feed);
            source_frames_fed += fed;

            self.scratch.resize(engine_frames * 2, 0.0);
            self.processor
                .process(&mut self.scratch[..engine_frames * 2]);

            self.scratch_l.resize(engine_frames, 0.0);
            self.scratch_r.resize(engine_frames, 0.0);
            for i in 0..engine_frames {
                self.scratch_l[i] = self.scratch[i * 2];
                self.scratch_r[i] = self.scratch[i * 2 + 1];
            }

            let cap = engine_frames.saturating_mul(4).saturating_add(512);
            self.resampled_l.clear();
            self.resampled_l.reserve(cap);
            self.resampled_r.clear();
            self.resampled_r.reserve(cap);
            self.resampler_l
                .process_into(&self.scratch_l, pitch, &mut self.resampled_l)
                .expect("resampler output reserved to max pitch-down emission");
            self.resampler_r
                .process_into(&self.scratch_r, pitch, &mut self.resampled_r)
                .expect("resampler output reserved to max pitch-down emission");

            let n = self.resampled_l.len().min(self.resampled_r.len());
            for i in 0..n {
                self.carry.push(self.resampled_l[i]);
                self.carry.push(self.resampled_r[i]);
            }

            let carried_after = (self.carry.len() - self.carry_head) / 2;
            if carried_after == carried_before && fed == 0 {
                break; // source dry and nothing emitted: avoid spinning
            }
        }

        let available = self.carry.len() - self.carry_head;
        let copy = available.min(need_samples);
        output[..copy].copy_from_slice(&self.carry[self.carry_head..self.carry_head + copy]);
        if copy < need_samples {
            output[copy..need_samples].fill(0.0);
        }
        self.carry_head += copy;
        if self.carry_head > CARRY_COMPACT_THRESHOLD || self.carry_head >= self.carry.len() {
            self.carry.drain(..self.carry_head);
            self.carry_head = 0;
        }

        StretchPullStats {
            source_frames_fed,
            out_frames,
        }
    }
}

impl TimeStretcher for TimestretchStretcher {
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn set_tempo_rate(&mut self, rate: f64) {
        let rate = if rate.is_finite() && rate > 0.0 {
            rate
        } else {
            1.0
        };
        self.tempo_rate = rate;
        self.controller.set_tempo_rate(self.engine_tempo());
    }

    fn set_pitch_factor(&mut self, factor: f64) {
        let factor = if factor.is_finite() {
            factor.clamp(MIN_PITCH_FACTOR, MAX_PITCH_FACTOR)
        } else {
            1.0
        };
        if (factor - self.pitch).abs() < f64::EPSILON {
            return;
        }
        self.pitch = factor;
        self.resampler_l.reset();
        self.resampler_r.reset();
        self.resampler_l.set_step_anchor(factor);
        self.resampler_r.set_step_anchor(factor);
        self.carry.clear();
        self.carry_head = 0;
        self.controller.set_tempo_rate(self.engine_tempo());
    }

    fn preferred_start_pad(&self) -> usize {
        0
    }

    fn start_delay(&self) -> usize {
        self.processor.pipeline_latency_frames() + self.resampler_l.group_delay_samples()
    }

    fn queued_source_frames(&self) -> usize {
        self.source.occupied_frames()
    }

    fn reset(&mut self) {
        self.processor.reset();
        self.controller.set_keylock(true);
        self.controller.set_tempo_rate(self.engine_tempo());
        self.resampler_l.reset();
        self.resampler_r.reset();
        self.resampler_l.set_step_anchor(self.pitch);
        self.resampler_r.set_step_anchor(self.pitch);
        self.carry.clear();
        self.carry_head = 0;
    }

    fn pull_interleaved(
        &mut self,
        out_frames: usize,
        output: &mut [Sample],
        feed: &mut dyn FnMut(usize, &mut [Sample]) -> usize,
    ) -> StretchPullStats {
        let need_samples = out_frames * 2;
        if output.len() < need_samples || out_frames == 0 {
            return StretchPullStats::default();
        }

        if self.pitch_engaged() {
            return self.pull_pitched(out_frames, output, feed);
        }

        let source_frames_fed = self.top_up(out_frames, feed);
        let out = &mut output[..need_samples];
        self.processor.process(out);

        StretchPullStats {
            source_frames_fed,
            out_frames,
        }
    }
}
