use std::path::PathBuf;

use thiserror::Error;

/// Bundle load / validation failure.
#[derive(Debug, Error)]
pub enum LoadError {
    #[error("unsupported schema_version {version} (expected 1)")]
    Schema { version: u32 },

    #[error("missing required file `{0}`")]
    MissingFile(PathBuf),

    #[error("failed to read `{path}`: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse `{path}`: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("{0}")]
    Validation(String),

    #[error("script compile error: {0}")]
    ScriptCompile(String),

    #[error("script `{0}` referenced but script.rhai is missing")]
    MissingScript(String),
}

/// Non-fatal runtime failure (e.g. Rhai panic while handling one event).
#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("script runtime error: {0}")]
    Script(String),
}

/// MIDI port I/O failure (host adapter).
#[derive(Debug, Error)]
pub enum MidiPortError {
    #[error("{0}")]
    Message(String),
}
