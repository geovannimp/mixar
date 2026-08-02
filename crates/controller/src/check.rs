//! Static bundle checks used by `map-check` and tests.

use std::path::Path;

use crate::bundle::load_bundle;
use crate::error::LoadError;
use crate::script::ScriptRuntime;

/// Load + validate a bundle directory (including optional Rhai compile).
pub fn check_bundle_dir(dir: &Path) -> Result<(), LoadError> {
    let bundle = load_bundle(dir)?;
    if let Some(src) = &bundle.script_source {
        ScriptRuntime::compile(src)?;
        // Ensure referenced script fns exist (best-effort: compile is enough for v1).
        let _ = src;
    }
    // Re-check script refs already done in map.validate_against.
    Ok(())
}

/// Check every immediate subdirectory of `mappings_root`.
pub fn check_all_mappings(mappings_root: &Path) -> Result<Vec<String>, Vec<(String, LoadError)>> {
    let mut ok = Vec::new();
    let mut err = Vec::new();
    let read = match std::fs::read_dir(mappings_root) {
        Ok(r) => r,
        Err(e) => {
            return Err(vec![(
                mappings_root.display().to_string(),
                LoadError::Io {
                    path: mappings_root.to_path_buf(),
                    source: e,
                },
            )]);
        }
    };
    for entry in read.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        match check_bundle_dir(&path) {
            Ok(()) => ok.push(name),
            Err(e) => err.push((name, e)),
        }
    }
    if err.is_empty() {
        Ok(ok)
    } else {
        Err(err)
    }
}
