//! One-shot EP probe:
//! `cargo run -p analyzer-stems --example ep_probe --manifest-path crates/Cargo.toml`

use std::path::PathBuf;
use std::time::Instant;

use analyzer_stems::{preferred_ep_labels, preferred_execution_providers, FORCE_EP_ENV};
use anyhow::{anyhow, Context, Result};
use ort::session::builder::GraphOptimizationLevel;
use ort::session::Session;
use ort::value::Tensor;

fn main() -> Result<()> {
    println!("cascade={:?}", preferred_ep_labels());
    println!("force={:?}", std::env::var(FORCE_EP_ENV).ok());

    let path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var("HOME").unwrap_or_default())
                .join(".local/share/top.mixar.app/models/htdemucs_mixxx_v1-db37d131.onnx")
        });
    println!("model={}", path.display());
    if !path.is_file() {
        anyhow::bail!("missing model {}", path.display());
    }

    let labels = preferred_ep_labels();
    let eps = preferred_execution_providers();
    for (label, ep) in labels.into_iter().zip(eps) {
        let started = Instant::now();
        match try_ep(&path, label, ep) {
            Ok(()) => {
                println!("OK ep={label} elapsed_ms={}", started.elapsed().as_millis());
                return Ok(());
            }
            Err(err) => {
                eprintln!("FAIL ep={label}: {err:#}");
            }
        }
    }
    Err(anyhow!("no EP worked"))
}

fn try_ep(
    path: &std::path::Path,
    label: &str,
    ep: ort::ep::ExecutionProviderDispatch,
) -> Result<()> {
    let mut session = Session::builder()
        .context("builder")?
        .with_optimization_level(GraphOptimizationLevel::Level3)
        .context("opt")?
        .with_execution_providers([ep])
        .with_context(|| format!("register {label}"))?
        .commit_from_file(path)
        .with_context(|| format!("commit {label}"))?;

    let input_name = session
        .inputs()
        .first()
        .map(|i| i.name().to_owned())
        .context("no inputs")?;
    println!("  input={input_name}");

    let zeros = vec![0.0f32; 2 * 343_980];
    let input = Tensor::from_array(([1usize, 2, 343_980], zeros)).context("tensor")?;
    let _ = session
        .run(ort::inputs![input_name.as_str() => input])
        .with_context(|| format!("probe {label}"))?;
    Ok(())
}
