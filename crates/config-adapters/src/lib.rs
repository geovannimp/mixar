//! Third-party application settings adapters for Mixar.
//!
//! [`library-adapters`] migrates music libraries; this crate migrates the
//! *settings* of other DJ applications into Mixar's native configuration types
//! (and writes them back). Adapters are pure, settings-time code: they run when
//! a migration is requested, never from the audio callback, and own no audio
//! state.
//!
//! [`library-adapters`]: https://github.com/geovannimp/mixar/tree/main/crates/library-adapters
//!
//! # Implemented adapters
//!
//! - `virtualdj` — VirtualDJ crossfader options (`crossfaderCurve`,
//!   `crossfaderCustom`, `crossfaderHamster`, `crossfaderDisable`).
//!
//! Enable an adapter with its Cargo feature; `virtualdj` is enabled by default.

#[cfg(feature = "virtualdj")]
pub mod virtualdj;

#[cfg(feature = "virtualdj")]
pub use virtualdj::{
    export_crossfader, import_crossfader, import_crossfader_options, CrossfaderConfig,
    VirtualDjCrossfaderConfig, VirtualDjError,
};
