//! Third-party library adapters for Mixar.
//!
//! Each adapter implements [`library_core::Library`] (and optionally
//! [`library_core::WritableLibrary`]) so external DJ software can be browsed
//! and imported into the user’s library manager (`library::LibraryManager`).
//!
//! Adapters are added as modules over time (Rekordbox, Serato, Traktor,
//! VirtualDJ, Engine DJ, …). Enable them via Cargo features when implemented.
//!
//! # Planned modules
//!
//! - `rekordbox` — Rekordbox XML / database
//! - `serato` — Serato `database V2` and crates
//! - `traktor` — Traktor `collection.nml`
//! - `virtualdj` — VirtualDJ lists
//! - `engine` — Engine DJ / Engine Library

// Future:
// #[cfg(feature = "rekordbox")]
// pub mod rekordbox;
