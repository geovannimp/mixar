//! Static bundle checks used by `map-check` and tests.

use std::path::Path;

use crate::bundle::load_bundle;
use crate::error::LoadError;
use crate::script::ScriptRuntime;

/// Load + validate a bundle directory (including optional Rhai compile).
pub fn check_bundle_dir(dir: &Path) -> Result<(), LoadError> {
    let bundle = load_bundle(dir)?;

    // A binding that names a script function but ships no `script.rhai` is
    // already rejected by `MapFile::validate_against` as `MissingScript`.
    let Some(src) = &bundle.script_source else {
        return Ok(());
    };
    let rt = ScriptRuntime::compile(src)?;

    // Lifecycle hooks are only invoked when named, so a typo silently no-ops at
    // runtime (`call_hook` treats a missing function as "optional"); this static
    // check is the only place to catch it.
    for (key, name) in [
        ("on_init", bundle.map.lifecycle.on_init.as_deref()),
        ("on_shutdown", bundle.map.lifecycle.on_shutdown.as_deref()),
        (
            "idle_heartbeat",
            bundle.map.lifecycle.idle_heartbeat.as_deref(),
        ),
    ] {
        if let Some(name) = name {
            require_fn(&rt, &format!("lifecycle.{key}"), name)?;
        }
    }

    // Input bindings that name a script function must resolve too. Without this,
    // `map-check` passes and the typo only surfaces as a `ScriptBindingFailure`
    // the first time the control is pressed.
    //
    // `bindings_for` is the single Action/Table/List normalization point, shared
    // with `MapFile::validate_against`; an `Action` carries no `script`, so it
    // needs no check here.
    for (section, aliases) in &bundle.map.inputs {
        for alias in aliases.keys() {
            for (i, binding) in bundle.map.bindings_for(section, alias).iter().enumerate() {
                if let Some(name) = &binding.script {
                    require_fn(&rt, &format!("inputs.{section}.{alias}[{i}]"), name)?;
                }
            }
        }
    }

    Ok(())
}

/// Reject a lifecycle hook or input binding that names a function `script.rhai`
/// does not define. `path` is the human-readable location used in the error.
fn require_fn(rt: &ScriptRuntime, path: &str, name: &str) -> Result<(), LoadError> {
    if rt.has_fn(name) {
        Ok(())
    } else {
        Err(LoadError::Validation(format!(
            "{path}: script function `{name}` not found in script.rhai"
        )))
    }
}

/// Check every immediate subdirectory of `mappings_root`.
///
/// Errors if the root holds no bundle directories at all: a wrong path used to
/// report "0 bundle(s) ok" and exit successfully, so CI never noticed.
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
    let mut bundle_dirs = 0usize;
    for entry in read {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                err.push((
                    mappings_root.display().to_string(),
                    LoadError::Io {
                        path: mappings_root.to_path_buf(),
                        source: e,
                    },
                ));
                continue;
            }
        };
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        bundle_dirs += 1;
        let name = entry.file_name().to_string_lossy().into_owned();
        match check_bundle_dir(&path) {
            Ok(()) => ok.push(name),
            Err(e) => err.push((name, e)),
        }
    }
    if bundle_dirs == 0 {
        err.push((
            mappings_root.display().to_string(),
            LoadError::Validation(format!(
                "no mapping bundle directories found in `{}`",
                mappings_root.display()
            )),
        ));
    }
    if err.is_empty() {
        Ok(ok)
    } else {
        Err(err)
    }
}
