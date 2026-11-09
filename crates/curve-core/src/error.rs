use thiserror::Error;

/// A curve, group, binding, or library validation failure.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum CurveError {
    #[error("a curve needs between 2 and 256 points, got {count}")]
    PointCount { count: usize },

    #[error("a curve with {points} points needs {expected} segments, got {count}")]
    SegmentCount {
        points: usize,
        expected: usize,
        count: usize,
    },

    #[error("point {index} has a non-finite coordinate ({x}, {y})")]
    NonFinitePoint { index: usize, x: f32, y: f32 },

    #[error("point {index} x={x} is outside 0..=1")]
    XOutOfRange { index: usize, x: f32 },

    #[error("point {index} y={y} is outside 0..=1")]
    YOutOfRange { index: usize, y: f32 },

    #[error("point {index} x={x} is not greater than the previous x={previous}")]
    NonIncreasingX { index: usize, x: f32, previous: f32 },

    #[error("curve must start at x=0, got {x}")]
    MissingStart { x: f32 },

    #[error("curve must end at x=1, got {x}")]
    MissingEnd { x: f32 },

    #[error("segment {index} rate {rate} must be finite with an absolute value of at most 20")]
    RateOutOfRange { index: usize, rate: f32 },

    #[error("segment {index} Bézier control point ({x}, {y}) is outside 0..=1")]
    BezierOutOfRange { index: usize, x: f32, y: f32 },

    #[error("curve {index}: {source}")]
    Curve {
        index: usize,
        #[source]
        source: Box<CurveError>,
    },

    #[error("curve group `{id}`: {source}")]
    NamedGroup {
        id: String,
        #[source]
        source: Box<CurveError>,
    },

    #[error("curve set `{id}`: {source}")]
    NamedSet {
        id: String,
        #[source]
        source: Box<CurveError>,
    },

    #[error("a curve group must hold between 1 and 16 curves, got {count}")]
    GroupCardinality { count: usize },

    #[error("this target needs {expected} curves, but the group has {count}")]
    ChannelCount { expected: usize, count: usize },

    #[error("expected an output buffer of {expected} values, got {actual}")]
    OutputLength { expected: usize, actual: usize },

    #[error("a resolved curve group needs at least one compiled member")]
    EmptyResolvedGroup,

    #[error("resolved curve groups mix {expected} and {count} channels")]
    ResolvedChannelMismatch { expected: usize, count: usize },

    #[error("`{id}` is not a snake_case id (expected [a-z][a-z0-9_]*, at most 64 characters)")]
    InvalidId { id: String },

    #[error("`{id}` is reserved for a built-in preset")]
    ReservedId { id: String },

    #[error("preset `{id}` needs a finite parameter, got {param}")]
    NonFiniteParameter { id: String, param: f32 },

    #[error("duplicate curve group id `{id}`")]
    DuplicateGroupId { id: String },

    #[error("duplicate curve set id `{id}`")]
    DuplicateSetId { id: String },

    #[error("a curve library holds at most 64 custom groups, got {count}")]
    TooManyGroups { count: usize },

    #[error("a curve library holds at most 32 sets, got {count}")]
    TooManySets { count: usize },

    #[error("curve set `{id}` holds at most 32 members, got {count}")]
    TooManySetMembers { id: String, count: usize },

    #[error("curve set `{id}` needs at least one member")]
    EmptySet { id: String },

    #[error("curve set `{id}` references unknown custom group `{group_id}`")]
    UnknownGroup { id: String, group_id: String },

    #[error("axis bounds must be finite with min < max, got {min}..{max}")]
    AxisBounds { min: f32, max: f32 },

    #[error("a logarithmic axis needs positive bounds, got {min}..{max}")]
    LogarithmicAxisBounds { min: f32, max: f32 },
}
