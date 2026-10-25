//! Test-only seams (`test-utils` feature, plus `cfg(test)` for in-crate use).

use crate::api::engine::EngineTransport;
use crate::api::library::LibraryTransport;

/// Start an [`EngineTransport`] with an explicit engine config.
///
/// Integration tests use this to run on the headless `null` backend. It lives
/// outside `crate::api`, so the Dart bridge never exposes a backend selector:
/// production always boots the compiled-in real backend from the settings host.
pub fn start_engine(
    library_transport: &LibraryTransport,
    config: engine_core::EngineConfig,
) -> Result<EngineTransport, String> {
    EngineTransport::start_with(library_transport, config)
}
