pub mod api;
mod app_paths;
mod bridge;
#[allow(clippy::wildcard_imports, clippy::unwrap_used)] // generated code
mod frb_generated;

#[cfg(any(test, feature = "test-utils"))]
#[doc(hidden)]
pub mod testing;
