//! Third-party library adapters for Mixar.
//!
//! Each adapter implements [`library_core::Library`] (and [`library_core::Migratable`])
//! so external DJ software can be browsed and imported into the user’s library
//! manager (`library::LibraryManager`).
//!
//! Adapters are added as modules over time (Mixxx, Rekordbox, Serato, Traktor,
//! VirtualDJ, Engine DJ, …). Enable them via Cargo features when implemented.
//!
//! # Implemented modules
//!
//! - `mixxx` — read-only view over a Mixxx `mixxxdb.sqlite` and migration into
//!   the user’s library.
//! - `cdj` — export into the Pioneer CDJ/XDJ USB layout (DeviceSQL database
//!   plus ANLZ analysis files).

#[cfg(feature = "cdj")]
pub mod cdj;

#[cfg(feature = "mixxx")]
pub mod mixxx;

#[cfg(feature = "cdj")]
pub use cdj::CdjExporter;

#[cfg(feature = "mixxx")]
pub use mixxx::{default_database_path, MixxxLibrary, MixxxPreview};
