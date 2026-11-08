//! Target-agnostic curve engine for normalized controls.
//!
//! A curve maps the normalized domain `0..=1` to one or more normalized
//! outputs. It has no knowledge of audio, DSP graphs, settings, controllers, or
//! any third-party configuration format: callers resolve ids, bind real-world
//! ranges with [`CurveBinding`], and compile [`CurveTable`]s for their own
//! runtime.
//!
//! # Model
//!
//! [`CurveSpec`] is a list of [`CurvePoint`]s plus one [`SegmentKind`] per
//! adjacent pair. Within a segment the shape is normalized to `0..=1` and
//! applied to the segment's `y` span, so every segment starts and ends exactly
//! on its points:
//!
//! - [`SegmentKind::Linear`] — `f(t) = t`.
//! - [`SegmentKind::Exponential`] — signed rate, `|rate| <= 20`; near zero
//!   falls back to linear.
//! - [`SegmentKind::Logarithmic`] — the inverse taper of the exponential shape.
//! - [`SegmentKind::CubicBezier`] — CSS-like control points in `0..=1`.
//!
//! A [`CurveGroup`] holds one curve per output (a scalar control uses one, a
//! two-channel control uses two). [`CurveTable`] samples a validated curve into
//! [`TABLE_SAMPLES`] values and evaluates it with linear interpolation.
//!
//! ```no_run
//! use curve_core::{presets, AxisMap, CurveBinding, CurveGroup, CurveGroupRef};
//!
//! let curve = presets::exponential(4.0).expect("valid rate");
//! let group = CurveGroup::new(vec![curve]).expect("one channel");
//! let binding = CurveBinding::new(
//!     CurveGroupRef::preset("exponential"),
//!     AxisMap::linear(-8.0, 8.0).expect("valid input range"),
//!     AxisMap::linear(-8.0, 8.0).expect("valid output range"),
//! )
//! .expect("valid binding");
//! let tempo = binding
//!     .evaluate_scalar(&group, 0.0)
//!     .expect("one-channel group");
//! assert_eq!(tempo, 0.0);
//! ```

mod axis;
mod binding;
mod error;
mod group;
mod library;
mod point;
pub mod presets;
mod refs;
mod segment;
mod spec;
mod table;

pub use axis::{AxisMap, AxisScale};
pub use binding::CurveBinding;
pub use error::CurveError;
pub use group::{CompiledCurveGroup, CurveGroup, MAX_GROUP_CHANNELS};
pub use library::{
    CurveLibrary, CurveSet, NamedCurveGroup, MAX_CUSTOM_GROUPS, MAX_SETS, MAX_SET_MEMBERS,
};
pub use point::CurvePoint;
pub use refs::{CurveGroupRef, CurveRefKind, MAX_ID_LEN};
pub use segment::{SegmentKind, MAX_SEGMENT_RATE};
pub use spec::{CurveSpec, MAX_POINTS, MIN_POINTS};
pub use table::{CurveTable, TABLE_SAMPLES};
