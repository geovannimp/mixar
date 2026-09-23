//! Process-global ort Session for Mixxx HTDemucs ONNX.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use anyhow::{anyhow, bail, Context, Result};
use ort::session::builder::GraphOptimizationLevel;
use ort::session::Session;
use tracing::{info, warn};

use crate::ep::preferred_ep_cascade;

/// Fixed HTDemucs segment length (samples per channel @ 44.1 kHz).
pub const SEGMENT_SAMPLES: usize = 343_980;

struct LoadedSession {
    path: PathBuf,
    session: Session,
    ep_label: &'static str,
    input_name: String,
    output_name: String,
}

static SESSION: OnceLock<Mutex<Option<LoadedSession>>> = OnceLock::new();

fn slot() -> &'static Mutex<Option<LoadedSession>> {
    SESSION.get_or_init(|| Mutex::new(None))
}

/// Ensure a process-global Session for `onnx_path`.
///
/// Callback receives `(session, ep_label, input_name, output_name)`.
pub fn with_session<R>(
    onnx_path: &Path,
    f: impl FnOnce(&mut Session, &'static str, &str, &str) -> Result<R>,
) -> Result<R> {
    let mut guard = slot().lock().unwrap_or_else(|e| e.into_inner());
    if guard.as_ref().is_none_or(|s| s.path != onnx_path) {
        *guard = Some(load_session(onnx_path)?);
    }
    let loaded = guard.as_mut().expect("session just loaded");
    f(
        &mut loaded.session,
        loaded.ep_label,
        &loaded.input_name,
        &loaded.output_name,
    )
}

fn load_session(path: &Path) -> Result<LoadedSession> {
    let started = Instant::now();
    let cascade = preferred_ep_cascade();
    info!(
        path = %path.display(),
        cascade = ?cascade.iter().map(|(l, _)| *l).collect::<Vec<_>>(),
        "stem ort: loading ONNX"
    );

    let mut last_err = None;

    for (label, ep) in cascade {
        match try_commit(path, label, ep) {
            Ok(mut session) => {
                let (input_name, output_name) = match resolve_io_names(&session) {
                    Ok(names) => names,
                    Err(err) => {
                        warn!(ep = label, error = %format!("{err:#}"), "stem ort: bad I/O; trying next");
                        last_err = Some(err);
                        continue;
                    }
                };
                if let Err(err) = probe_session(&mut session, &input_name) {
                    let detail = format!("{err:#}");
                    if detail.contains("leading 0") || detail.contains("0,00") {
                        warn!(
                            ep = label,
                            error = %detail,
                            "stem ort: WebGPU WGSL float locale bug; trying next EP"
                        );
                    } else if detail.contains("WGSL") {
                        warn!(
                            ep = label,
                            error = %detail,
                            "stem ort: WebGPU WGSL validation failed; trying next EP"
                        );
                    } else {
                        warn!(
                            ep = label,
                            error = %detail,
                            "stem ort: EP probe failed; trying next"
                        );
                    }
                    last_err = Some(err);
                    continue;
                }
                info!(
                    path = %path.display(),
                    ep = label,
                    input = %input_name,
                    output = %output_name,
                    elapsed_ms = started.elapsed().as_millis() as u64,
                    "stem ort: session ready"
                );
                return Ok(LoadedSession {
                    path: path.to_path_buf(),
                    session,
                    ep_label: label,
                    input_name,
                    output_name,
                });
            }
            Err(err) => {
                warn!(
                    ep = label,
                    error = %format!("{err:#}"),
                    "stem ort: EP session commit failed; trying next"
                );
                last_err = Some(err);
            }
        }
    }

    Err(last_err
        .unwrap_or_else(|| anyhow!("no ORT execution provider could load {}", path.display())))
}

fn try_commit(path: &Path, label: &str, ep: ort::ep::ExecutionProviderDispatch) -> Result<Session> {
    info!(ep = label, "stem ort: trying EP");
    #[cfg(feature = "webgpu")]
    if label == "webgpu" {
        // Re-assert before commit: Flutter (or anything else) may have reset locale.
        crate::ep::force_c_numeric_locale();
    }

    Session::builder()
        .context("ort Session::builder")?
        .with_optimization_level(GraphOptimizationLevel::Level3)
        .context("ort optimization level")?
        .with_execution_providers([ep])
        .with_context(|| format!("ort register EP '{label}'"))?
        .commit_from_file(path)
        .with_context(|| format!("ort commit ONNX with EP '{label}' ({})", path.display()))
}

/// Mixxx/stemgen uses `input`/`output` (discovered dynamically from the graph).
pub(crate) fn resolve_io_names(session: &Session) -> Result<(String, String)> {
    let inputs = session.inputs();
    let outputs = session.outputs();
    if inputs.len() != 1 {
        bail!("expected 1 ONNX input, got {}", inputs.len());
    }
    if outputs.len() != 1 {
        bail!("expected 1 ONNX output, got {}", outputs.len());
    }
    Ok((inputs[0].name().to_owned(), outputs[0].name().to_owned()))
}

/// One zero-input forward so unsupported ops fail before real audio.
fn probe_session(session: &mut Session, input_name: &str) -> Result<()> {
    use ort::value::Tensor;

    let zeros = vec![0.0f32; 2 * SEGMENT_SAMPLES];
    let input =
        Tensor::from_array(([1usize, 2, SEGMENT_SAMPLES], zeros)).context("ort probe tensor")?;
    let _outputs = session
        .run(ort::inputs![input_name => input])
        .context("ort probe run")?;
    Ok(())
}
