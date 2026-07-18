use thiserror::Error;

pub type Result<T> = std::result::Result<T, AnalyzerError>;

#[derive(Debug, Error)]
pub enum AnalyzerError {
    #[error("decode failed: {0}")]
    Decode(String),
    #[error("analysis failed: {0}")]
    Analysis(String),
    #[error("unsupported: {0}")]
    Unsupported(&'static str),
    #[error("backend {backend}: {message}")]
    Backend {
        backend: &'static str,
        message: String,
    },
}
