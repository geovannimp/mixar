//! Test-only seams (`test-utils` feature, plus `cfg(test)` for in-crate use).

use std::path::Path;

use crate::api::bus::BusTransport;
use crate::api::controller::ControllerTransport;
use crate::api::engine::EngineTransport;
use crate::api::library::LibraryTransport;
use bus_api::Event;
use event_bus::Subscriber;

/// Start an [`EngineTransport`] with an explicit engine config.
///
/// Integration tests use this to run on the headless `null` backend. It lives
/// outside `crate::api`, so the Dart bridge never exposes a backend selector:
/// production always boots the compiled-in real backend from the settings host.
pub fn start_engine(
    bus: &BusTransport,
    library_transport: &LibraryTransport,
    config: engine_core::EngineConfig,
) -> Result<EngineTransport, String> {
    EngineTransport::start_with(bus.bus(), bus.deck_chrome(), library_transport, config)
}

/// Subscribe to every bus event, for integration tests that assert on egress.
pub fn subscribe_evt(bus: &BusTransport) -> Result<Subscriber<Event>, String> {
    bus.subscribe_evt()
}

/// Open a library at an explicit path, so tests use a temp directory instead of
/// the host's app-support one (production resolves that in
/// [`crate::app_paths`]).
pub fn open_library_at(db_path: &Path) -> Result<LibraryTransport, String> {
    LibraryTransport::open_at_on_own_bus(db_path)
}

/// Start the controller host against explicit mapping directories.
pub fn start_controller_at(
    bus: &BusTransport,
    mappings_dir: &Path,
    shipped_mappings_dir: Option<&Path>,
    trusted_device_ids: Vec<String>,
) -> Result<ControllerTransport, String> {
    ControllerTransport::start_at(
        bus,
        mappings_dir.to_path_buf(),
        shipped_mappings_dir.map(Path::to_path_buf),
        trusted_device_ids,
    )
}
