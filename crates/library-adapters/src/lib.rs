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

#[cfg(feature = "mixxx")]
pub mod mixxx;

#[cfg(feature = "mixxx")]
pub use mixxx::{default_database_path, MixxxLibrary, MixxxPreview};
