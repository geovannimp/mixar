use anyhow::Result;

/// Create backend by name (used by Engine and by AudioBackend factory).
pub(crate) fn create_backend(backend_name: &str) -> Result<Box<dyn audio_core::AudioBackend>> {
    match backend_name {
        "null" => {
            let backend = backend_null::NullBackend::new();
            Ok(Box::new(backend))
        }
        "miniaudio" => {
            let backend = backend_miniaudio::MiniaudioBackend::new()?;
            Ok(Box::new(backend))
        }
        "cpal" => {
            #[cfg(feature = "backend-cpal")]
            {
                let backend = backend_cpal::CpalBackend::new()?;
                Ok(Box::new(backend))
            }
            #[cfg(not(feature = "backend-cpal"))]
            Err(anyhow::anyhow!(
                "CPAL backend not compiled in. Build with default features or enable 'backend-cpal'."
            ))
        }
        "auto" => {
            #[cfg(feature = "backend-cpal")]
            match backend_cpal::CpalBackend::new() {
                Ok(backend) => {
                    log::info!("Using CPAL backend");
                    return Ok(Box::new(backend));
                }
                Err(e) => log::warn!("Failed to initialize CPAL backend: {}, trying miniaudio", e),
            }
            match backend_miniaudio::MiniaudioBackend::new() {
                Ok(backend) => {
                    log::info!("Using miniaudio backend");
                    Ok(Box::new(backend))
                }
                Err(e) => {
                    log::warn!(
                        "Failed to initialize miniaudio backend: {}, falling back to null backend",
                        e
                    );
                    let backend = backend_null::NullBackend::new();
                    Ok(Box::new(backend))
                }
            }
        }
        _ => Err(anyhow::anyhow!("Unknown backend: {}", backend_name)),
    }
}

/// Factory for listing and creating audio backends. Use this to discover
/// backends and devices before building config and creating an engine.
pub struct AudioBackend;

impl AudioBackend {
    /// Returns the list of available backend names (e.g. `["null", "miniaudio", "cpal"]`).
    /// Use one of these with `AudioBackend::new()` and for `EngineConfig::backend` (or use `"auto"` for config).
    pub fn list_names() -> Vec<String> {
        let mut backends = vec!["null".to_string(), "miniaudio".to_string()];
        #[cfg(feature = "backend-cpal")]
        backends.push("cpal".to_string());
        backends
    }

    /// Creates a backend instance by name. Use `list_names()` for valid names.
    /// Returns a boxed backend on which you can call `list_output_devices()` (devices include `is_default`).
    /// Bring the `AudioBackendTrait` trait into scope to call those methods.
    #[allow(clippy::new_ret_no_self)]
    pub fn new(name: &str) -> Result<Box<dyn audio_core::AudioBackend>> {
        create_backend(name)
    }
}

/// Re-export of the backend trait so callers can use backend methods without depending on audio-core.
pub use audio_core::AudioBackend as AudioBackendTrait;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_list_names() {
        let names = AudioBackend::list_names();
        assert!(!names.is_empty());
        assert!(names.contains(&"null".to_string()));
        assert!(names.contains(&"miniaudio".to_string()));
    }

    #[test]
    fn test_backend_new_and_list_devices() {
        let backend = AudioBackend::new("null").unwrap();
        let devices = backend.list_output_devices();
        assert!(devices.is_ok());
        assert!(!devices.unwrap().is_empty());
    }

    #[test]
    fn test_backend_new_and_default_from_list() {
        let backend = AudioBackend::new("null").unwrap();
        let devices = backend.list_output_devices().unwrap();
        let default = devices.iter().find(|d| d.is_default).or(devices.first());
        assert!(default.is_some());
    }
}
