//! Process-global ort Session for StemSplit HTDemucs ONNX.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use anyhow::{anyhow, Context, Result};
use ort::session::builder::GraphOptimizationLevel;
use ort::session::Session;
use tracing::{info, warn};

use crate::ep::preferred_ep_cascade;

/// Fixed StemSplit segment length (samples per channel @ 44.1 kHz).
pub const SEGMENT_SAMPLES: usize = 343_980;

struct LoadedSession {
    path: PathBuf,
    session: Session,
    ep_label: &'static str,
}

static SESSION: OnceLock<Mutex<Option<LoadedSession>>> = OnceLock::new();

fn slot() -> &'static Mutex<Option<LoadedSession>> {
    SESSION.get_or_init(|| Mutex::new(None))
}

/// Ensure a process-global Session for `onnx_path`, returning the EP label used.
pub fn with_session<R>(
    onnx_path: &Path,
    f: impl FnOnce(&mut Session, &'static str) -> Result<R>,
) -> Result<R> {
    let mut guard = slot().lock().unwrap_or_else(|e| e.into_inner());
    if guard.as_ref().is_none_or(|s| s.path != onnx_path) {
        *guard = Some(load_session(onnx_path)?);
    }
    let loaded = guard.as_mut().expect("session just loaded");
    f(&mut loaded.session, loaded.ep_label)
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
                if let Err(err) = probe_session(&mut session) {
                    warn!(
                        ep = label,
                        error = %err,
                        "stem ort: EP probe failed; trying next"
                    );
                    last_err = Some(err);
                    continue;
                }
                info!(
                    path = %path.display(),
                    ep = label,
                    elapsed_ms = started.elapsed().as_millis() as u64,
                    "stem ort: session ready"
                );
                return Ok(LoadedSession {
                    path: path.to_path_buf(),
                    session,
                    ep_label: label,
                });
            }
            Err(err) => {
                warn!(
                    ep = label,
                    error = %err,
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
    Session::builder()
        .context("ort Session::builder")?
        .with_optimization_level(GraphOptimizationLevel::Level3)
        .context("ort optimization level")?
        .with_execution_providers([ep])
        .with_context(|| format!("ort register EP '{label}'"))?
        .commit_from_file(path)
        .with_context(|| format!("ort commit ONNX with EP '{label}' ({})", path.display()))
}

/// One zero-input forward so unsupported ops fail before real audio.
fn probe_session(session: &mut Session) -> Result<()> {
    use ort::value::Tensor;

    let zeros = vec![0.0f32; 2 * SEGMENT_SAMPLES];
    let input =
        Tensor::from_array(([1usize, 2, SEGMENT_SAMPLES], zeros)).context("ort probe tensor")?;
    let _outputs = session
        .run(ort::inputs!["mix" => input])
        .context("ort probe run")?;
    Ok(())
}
