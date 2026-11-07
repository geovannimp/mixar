//! StemgenRT live throughput per execution provider (on-device benchmark).
//!
//! ```text
//! cargo build -p analyzer-stems --target aarch64-linux-android --example live_ep_bench
//! adb push target/aarch64-linux-android/debug/examples/live_ep_bench /data/local/tmp/
//! adb push libonnxruntime.so model.onnx /data/local/tmp/
//! adb shell ORT_DYLIB_PATH=/data/local/tmp/libonnxruntime.so \
//!   /data/local/tmp/live_ep_bench /data/local/tmp/model.onnx 300
//! ```
//!
//! Prints `avg_hop_ms` and the realtime factor per provider; below 1.0 the
//! producer cannot stay ahead of playback.

use std::path::{Path, PathBuf};
use std::time::Instant;

use analyzer_stems::preferred_ep_labels;
#[cfg(not(target_os = "android"))]
use analyzer_stems::preferred_execution_providers;
use analyzer_stems::STEMGEN_AUDIO_INPUT;
use analyzer_stems::STEMGEN_STATE_OUTPUT_PREFIX;
#[cfg(target_os = "android")]
use analyzer_stems::{ANDROID_ORT_DYLIB, ORT_DYLIB_PATH_ENV};
use anyhow::{anyhow, bail, Context, Result};
use ort::environment::Environment;
use ort::ep::ExecutionProviderDispatch;
use ort::session::builder::GraphOptimizationLevel;
use ort::session::Session;
use ort::value::{Tensor, ValueType};

/// StemgenRT hop (samples per channel).
const HOP: usize = 128;
const SAMPLE_RATE: f64 = 44_100.0;
const WARMUP: usize = 10;

struct State {
    name: String,
    shape: Vec<usize>,
    data: Vec<f32>,
}

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let model = PathBuf::from(
        args.next()
            .context("usage: live_ep_bench <model.onnx> [hops]")?,
    );
    let hops: usize = args.next().map_or(Ok(300), |raw| raw.parse())?;
    // Optional override for thread-count experiments; defaults to the live path.
    let threads: Option<usize> = args.next().map(|raw| raw.parse()).transpose()?;

    #[cfg(target_os = "android")]
    let builder = {
        let path =
            std::env::var(ORT_DYLIB_PATH_ENV).unwrap_or_else(|_| ANDROID_ORT_DYLIB.to_owned());
        ort::init_from(&path).with_context(|| format!("load {path}"))?
    };
    #[cfg(not(target_os = "android"))]
    let builder = ort::init();
    let env = builder.build().context("ort environment")?;

    println!(
        "model={} hops={} threads={:?} cascade={:?}",
        model.display(),
        hops,
        threads,
        preferred_ep_labels()
    );

    for (label, ep) in candidates() {
        let Some(ep) = ep else {
            println!("ep={label} available=false");
            continue;
        };
        match run_ep(&env, &model, label, ep, threads, hops) {
            Ok((avg_ms, threads_used)) => {
                let budget_ms = HOP as f64 / SAMPLE_RATE * 1000.0;
                println!(
                    "ep={label} threads={threads_used} available=true avg_hop_ms={avg_ms:.2} realtime={:.2}",
                    budget_ms / avg_ms
                );
            }
            Err(err) => println!("ep={label} available=true failed: {err:#}"),
        }
    }
    Ok(())
}

/// Every execution provider this build can name. On Android the full list is
/// probed against the loaded runtime, so a custom ONNX Runtime with ACL/QNN/TVM/
/// Azure is benchmarked without code changes; entries the runtime lacks print
/// `available=false`. Desktop keeps the regular cascade (WebGPU/CPU).
fn candidates() -> Vec<(&'static str, Option<ExecutionProviderDispatch>)> {
    #[cfg(target_os = "android")]
    {
        use ort::ep::ExecutionProvider as _;

        macro_rules! probe {
            ($ty:ident, $label:expr) => {{
                let available = ort::ep::$ty::is_available().unwrap_or(false);
                ($label, available.then(|| ort::ep::$ty::default().build()))
            }};
        }

        vec![
            probe!(NNAPI, analyzer_stems::NNAPI_EP_LABEL),
            probe!(XNNPACK, analyzer_stems::XNNPACK_EP_LABEL),
            probe!(ACL, analyzer_stems::ACL_EP_LABEL),
            (analyzer_stems::QNN_EP_LABEL, qnn_ep()),
            probe!(TVM, analyzer_stems::TVM_EP_LABEL),
            probe!(Azure, analyzer_stems::AZURE_EP_LABEL),
            (
                analyzer_stems::CPU_EP_LABEL,
                Some(ort::ep::CPU::default().build()),
            ),
        ]
    }
    #[cfg(not(target_os = "android"))]
    {
        preferred_ep_labels()
            .into_iter()
            .zip(preferred_execution_providers())
            .map(|(label, ep)| (label, Some(ep)))
            .collect()
    }
}

/// QNN needs backend/perf tuning to be meaningful. Defaults mirror ORT (HTP
/// backend, fp16 as compiled), overridable per run:
/// `MIXAR_QNN_BACKEND`, `MIXAR_QNN_PERF` (burst/balanced/high/sustained),
/// `MIXAR_QNN_FP16` (0/1), `MIXAR_QNN_HTP_ARCH`.
#[cfg(target_os = "android")]
fn qnn_ep() -> Option<ExecutionProviderDispatch> {
    use ort::ep::ExecutionProvider as _;

    if !ort::ep::QNN::is_available().unwrap_or(false) {
        return None;
    }
    // Env tuning (MIXAR_QNN_PERF/FP16/BACKEND/HTP_ARCH) lives in the library so
    // the app and this bench build the same provider.
    analyzer_stems::execution_provider(analyzer_stems::QNN_EP_LABEL)
}

fn run_ep(
    env: &Environment,
    model: &Path,
    label: &str,
    ep: ExecutionProviderDispatch,
    threads: Option<usize>,
    hops: usize,
) -> Result<(f64, usize)> {
    // Shared with `stemgen::commit_session`, so the benchmark measures the
    // same thread policy production runs.
    let intra_threads = analyzer_stems::intra_threads_for(label, threads)?;
    let builder = Session::builder(env)
        .context("session builder")?
        .with_optimization_level(GraphOptimizationLevel::Level3)
        .map_err(|err| anyhow!("optimization level: {err}"))?
        .with_intra_threads(intra_threads)
        .map_err(|err| anyhow!("intra threads: {err}"))?;
    let mut session = builder
        .with_inter_threads(1)
        .map_err(|err| anyhow!("inter threads: {err}"))?
        .with_execution_providers([ep])
        .map_err(|err| anyhow!("register EP: {err}"))?
        .commit_from_file(model)
        .with_context(|| format!("load {}", model.display()))?;

    let mut states = Vec::new();
    for input in session.inputs() {
        if input.name() == STEMGEN_AUDIO_INPUT {
            continue;
        }
        let ValueType::Tensor { shape, .. } = input.dtype() else {
            bail!("state '{}' is not a tensor", input.name());
        };
        let dims: Vec<usize> = shape
            .iter()
            .map(|d| if *d > 0 { *d as usize } else { 1 })
            .collect();
        let len: usize = dims.iter().product();
        states.push(State {
            name: input.name().to_owned(),
            shape: dims,
            data: vec![0.0; len],
        });
    }

    let audio = vec![0.0f32; 2 * HOP];
    let hop = |session: &mut Session, states: &mut Vec<State>| -> Result<()> {
        let mut inputs: Vec<(String, Tensor<f32>)> = Vec::with_capacity(states.len() + 1);
        inputs.push((
            STEMGEN_AUDIO_INPUT.into(),
            Tensor::from_array(([1usize, 2, HOP], audio.clone())).context("audio tensor")?,
        ));
        for state in states.iter() {
            inputs.push((
                state.name.clone(),
                Tensor::from_array((state.shape.clone(), state.data.clone()))
                    .with_context(|| format!("state tensor '{}'", state.name))?,
            ));
        }
        let outputs = session.run(inputs).context("run")?;
        for state in states.iter_mut() {
            let name = format!("{}{}", STEMGEN_STATE_OUTPUT_PREFIX, state.name);
            let value = outputs
                .get(&name)
                .ok_or_else(|| anyhow!("missing output '{name}'"))?;
            let (_, next) = value.try_extract_tensor::<f32>().context("extract state")?;
            state.data.copy_from_slice(next);
        }
        Ok(())
    };

    for _ in 0..WARMUP {
        hop(&mut session, &mut states)?;
    }
    let started = Instant::now();
    for _ in 0..hops {
        hop(&mut session, &mut states)?;
    }
    Ok((
        started.elapsed().as_secs_f64() * 1000.0 / hops as f64,
        intra_threads,
    ))
}
