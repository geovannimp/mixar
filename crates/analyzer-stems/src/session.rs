//! Process-global ort Session for Mixxx HTDemucs ONNX.

use std::collections::HashMap;
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
    session: Session,
    ep_label: &'static str,
    input_name: String,
    output_name: String,
}

/// Keep one session per model path. Switching models (HTDemucs ↔ SCNet) must
/// not reload a several-hundred-MB graph on every chunk, and a live HTDemucs
/// producer may overlap an SCNet generation.
const MAX_CACHED_SESSIONS: usize = 2;

static SESSIONS: OnceLock<Mutex<HashMap<PathBuf, LoadedSession>>> = OnceLock::new();

fn slot() -> &'static Mutex<HashMap<PathBuf, LoadedSession>> {
    SESSIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Ensure a process-global Session for `onnx_path`.
///
/// Callback receives `(session, ep_label, input_name, output_name)`.
pub fn with_session<R>(
    onnx_path: &Path,
    f: impl FnOnce(&mut Session, &'static str, &str, &str) -> Result<R>,
) -> Result<R> {
    let mut guard = slot().lock().unwrap_or_else(|e| e.into_inner());
    if !guard.contains_key(onnx_path) {
        // Load first so a failed graph does not destroy healthy sessions.
        // Then enforce the bounded cache, evicting only after success.
        let loaded = load_session(onnx_path)?;
        if guard.len() >= MAX_CACHED_SESSIONS {
            guard.clear();
        }
        guard.insert(onnx_path.to_path_buf(), loaded);
    }
    let loaded = guard.get_mut(onnx_path).expect("session just loaded");
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
        let probe = || -> Result<(Session, String, String)> {
            let mut session = try_commit(path, label, ep)?;
            let (input_name, output_name) = resolve_io_names(&session)
                .with_context(|| format!("resolve ONNX I/O with EP '{label}'"))?;
            probe_session(&mut session, &input_name)?;
            Ok((session, input_name, output_name))
        };
        #[cfg(all(feature = "webgpu", unix))]
        let probe_result = if label == "webgpu" {
            crate::ep::with_c_numeric_locale(probe)
        } else {
            probe()
        };
        #[cfg(not(all(feature = "webgpu", unix)))]
        let probe_result = probe();
        match probe_result {
            Ok((session, input_name, output_name)) => {
                info!(
                    path = %path.display(),
                    ep = label,
                    input = %input_name,
                    output = %output_name,
                    elapsed_ms = started.elapsed().as_millis() as u64,
                    "stem ort: session ready"
                );
                return Ok(LoadedSession {
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
                    "stem ort: EP session failed; trying next"
                );
                last_err = Some(err);
            }
        }
    }

    Err(last_err
        .unwrap_or_else(|| anyhow!("no ORT execution provider could load {}", path.display())))
}

/// Build and commit a session for one EP attempt. The caller
/// ([`load_session`]) applies the single C-locale scope for the webgpu label,
/// so this must not re-enter `with_c_numeric_locale` — the internal
/// `LOCALE_LOCK` is non-reentrant.
fn try_commit(path: &Path, label: &str, ep: ort::ep::ExecutionProviderDispatch) -> Result<Session> {
    info!(ep = label, "stem ort: trying EP");
    let mut builder = Session::builder().context("ort Session::builder")?;
    builder = builder
        .with_optimization_level(GraphOptimizationLevel::Level3)
        .map_err(|err| anyhow!("ort optimization level: {err}"))?;
    builder = builder
        .with_execution_providers([ep])
        .map_err(|err| anyhow!("ort register EP '{label}': {err}"))?;
    builder
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
///
/// Uses the graph's declared input shape; HTDemucs is `[1,2,343980]` and
/// SCNet is `[1,4,2049,476]`.
fn probe_session(session: &mut Session, input_name: &str) -> Result<()> {
    use ort::value::Tensor;
    use ort::value::ValueType;

    let input = session
        .inputs()
        .first()
        .ok_or_else(|| anyhow!("ONNX graph has no inputs"))?;
    let ValueType::Tensor { shape, .. } = input.dtype() else {
        bail!("unsupported ONNX input type {:?}", input.dtype());
    };
    let dims: Vec<usize> = shape
        .iter()
        .map(|d| if *d > 0 { *d as usize } else { 1 })
        .collect();
    let len = dims.iter().product::<usize>();
    let zeros = vec![0.0f32; len];
    let tensor = Tensor::from_array((dims, zeros)).context("ort probe tensor")?;
    let _outputs = session
        .run(ort::inputs![input_name => tensor])
        .context("ort probe run")?;
    Ok(())
}
