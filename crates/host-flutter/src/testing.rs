//! Test-only seams (`test-utils` feature, plus `cfg(test)` for in-crate use).

use std::path::Path;

use crate::api::controller::ControllerTransport;
use crate::api::engine::{EngineBusHandle, EngineTransport};
use crate::api::library::{LibraryBusHandle, LibraryTransport};

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

/// Open a library at an explicit path, so tests use a temp directory instead of
/// the host's app-support one (production resolves that in
/// [`crate::app_paths`]).
pub fn open_library_at(db_path: &Path) -> Result<LibraryTransport, String> {
    LibraryTransport::open_at(db_path)
}

/// Start the controller host against explicit mapping directories.
pub fn start_controller_at(
    engine_buses: &EngineBusHandle,
    library_buses: &LibraryBusHandle,
    mappings_dir: &Path,
    shipped_mappings_dir: Option<&Path>,
    trusted_device_ids: Vec<String>,
) -> Result<ControllerTransport, String> {
    ControllerTransport::start_at(
        engine_buses,
        library_buses,
        mappings_dir.to_path_buf(),
        shipped_mappings_dir.map(Path::to_path_buf),
        trusted_device_ids,
    )
}
