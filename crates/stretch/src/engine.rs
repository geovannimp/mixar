//! timestretch realtime engine wrapper (WideKeylock = tempo without pitch).

use crate::{StretchPullStats, TimeStretcher};
use anyhow::{anyhow, Result};
use audio_core::Sample;
use timestretch::engine::{
    Engine, EngineConfig, EngineController, EngineProcessor, EngineProfile, SourceProducer,
    MAX_TEMPO_RATE,
};

/// timestretch WideKeylock stretcher for DJ key lock.
pub struct TimestretchStretcher {
    sample_rate: u32,
    controller: EngineController,
    processor: EngineProcessor,
    source: SourceProducer,
    feed_scratch: Vec<Sample>,
    tempo_rate: f64,
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

        Ok(Self {
            sample_rate,
            controller: handles.controller,
            processor: handles.processor,
            source: handles.source,
            feed_scratch: vec![0.0; max_block.max(512) * 2],
            tempo_rate: 1.0,
        })
    }

    fn top_up(
        &mut self,
        out_frames: usize,
        feed: &mut dyn FnMut(usize, &mut [Sample]) -> usize,
    ) -> usize {
        let demand = self
            .source
            .demand_hint(out_frames, self.tempo_rate.max(1.0));
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
        self.controller.set_tempo_rate(rate);
    }

    fn preferred_start_pad(&self) -> usize {
        0
    }

    fn start_delay(&self) -> usize {
        self.processor.pipeline_latency_frames()
    }

    fn reset(&mut self) {
        self.processor.reset();
        self.controller.set_keylock(true);
        self.controller.set_tempo_rate(self.tempo_rate);
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

        let source_frames_fed = self.top_up(out_frames, feed);
        let out = &mut output[..need_samples];
        self.processor.process(out);

        StretchPullStats {
            source_frames_fed,
            out_frames,
        }
    }
}
