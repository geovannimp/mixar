//! VirtualDJ `crossfaderCustom` point script parsing and formatting.

use curve_core::{CurvePoint, CurveSpec, SegmentKind};

use super::VirtualDjError;

/// Characters of the input shown in a parse error.
const ERROR_SNIPPET_CHARS: usize = 32;

/// One VirtualDJ crossfader point: a position and the two deck gains.
///
/// `x = 0` is deck A only, `x = 1` is deck B only. `a` and `b` are the output
/// gains of deck A and deck B at that position, both in `0..=1`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrossfaderPoint {
    pub x: f32,
    pub a: f32,
    pub b: f32,
}

impl CrossfaderPoint {
    pub const fn new(x: f32, a: f32, b: f32) -> Self {
        Self { x, a, b }
    }
}

/// Parse a VirtualDJ `crossfaderCustom` script.
///
/// The script lists `position=[gainA,gainB]` points. Points may be separated by
/// `/` and whitespace, or concatenated (`0=[1,0]0.5=[1,1]1=[0,1]`); both forms
/// are accepted. The result is validated to start at `x = 0`, end at `x = 1`,
/// and keep strictly increasing positions.
pub fn parse_points(input: &str) -> Result<Vec<CrossfaderPoint>, VirtualDjError> {
    let mut scanner = Scanner::new(input);
    let mut points = Vec::new();
    loop {
        scanner.skip_separators();
        if scanner.is_done() {
            break;
        }
        points.push(scanner.point()?);
    }
    validate_points(&points)?;
    Ok(points)
}

/// Format points as a VirtualDJ `crossfaderCustom` script (`x=[a,b]/…`).
///
/// Numbers use the shortest representation that roundtrips an `f32`.
pub fn format_points(points: &[CrossfaderPoint]) -> String {
    points
        .iter()
        .map(|point| {
            format!(
                "{}=[{},{}]",
                format_number(point.x),
                format_number(point.a),
                format_number(point.b)
            )
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn format_number(value: f32) -> String {
    if value == 0.0 {
        return "0".to_string();
    }
    if value == 1.0 {
        return "1".to_string();
    }
    value.to_string()
}

/// Build the native A/B curves from parsed VirtualDJ points.
///
/// Both curves share the point positions and use straight lines between them,
/// which is how VirtualDJ draws a custom curve.
pub(super) fn curves_from_points(
    points: &[CrossfaderPoint],
) -> Result<(CurveSpec, CurveSpec), VirtualDjError> {
    let segments = vec![SegmentKind::Linear; points.len() - 1];
    let a = CurveSpec::new(
        points
            .iter()
            .map(|point| CurvePoint::new(point.x, point.a))
            .collect(),
        segments.clone(),
    )?;
    let b = CurveSpec::new(
        points
            .iter()
            .map(|point| CurvePoint::new(point.x, point.b))
            .collect(),
        segments,
    )?;
    Ok((a, b))
}

fn validate_points(points: &[CrossfaderPoint]) -> Result<(), VirtualDjError> {
    let Some(first) = points.first().copied() else {
        return Err(VirtualDjError::TooFewPoints { count: 0 });
    };
    if points.len() < 2 {
        return Err(VirtualDjError::TooFewPoints { count: 1 });
    }
    if first.x != 0.0 {
        return Err(VirtualDjError::MissingStart { x: first.x });
    }
    let last = points[points.len() - 1];
    if last.x != 1.0 {
        return Err(VirtualDjError::MissingEnd { x: last.x });
    }

    let mut previous = first.x;
    for point in &points[1..] {
        if point.x <= previous {
            return Err(VirtualDjError::NonIncreasingPosition {
                x: point.x,
                previous,
            });
        }
        previous = point.x;
    }
    Ok(())
}

/// Cursor over the point script.
///
/// The grammar is small enough to scan without a tokenizer, and scanning keeps
/// the concatenated form unambiguous.
struct Scanner<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Scanner<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    fn peek(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let next = self.peek()?;
        self.pos += next.len_utf8();
        Some(next)
    }

    fn is_done(&self) -> bool {
        self.pos >= self.input.len()
    }

    /// Skip whitespace and the optional `/` point separator.
    fn skip_separators(&mut self) {
        while let Some(next) = self.peek() {
            if next.is_whitespace() || next == '/' {
                self.bump();
            } else {
                break;
            }
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(next) = self.peek() {
            if next.is_whitespace() {
                self.bump();
            } else {
                break;
            }
        }
    }

    fn expect(&mut self, expected: char, reason: &'static str) -> Result<(), VirtualDjError> {
        self.skip_whitespace();
        match self.bump() {
            Some(next) if next == expected => Ok(()),
            _ => Err(self.malformed(reason)),
        }
    }

    fn number(&mut self) -> Result<f32, VirtualDjError> {
        self.skip_whitespace();
        let start = self.pos;
        while let Some(next) = self.peek() {
            if next.is_ascii_digit() || matches!(next, '.' | '+' | '-' | 'e' | 'E') {
                self.bump();
            } else {
                break;
            }
        }
        if start == self.pos {
            return Err(self.malformed("expected a number"));
        }
        self.input[start..self.pos]
            .parse::<f32>()
            .map_err(|_| self.malformed("invalid number"))
    }

    fn point(&mut self) -> Result<CrossfaderPoint, VirtualDjError> {
        let x = self.number()?;
        self.expect('=', "expected `=` after the position")?;
        self.expect('[', "expected `[` after `=`")?;
        let a = self.number()?;
        self.expect(',', "expected `,` between the two deck gains")?;
        let b = self.number()?;
        self.expect(']', "expected `]` to close the point")?;

        if !(0.0..=1.0).contains(&x) {
            return Err(VirtualDjError::PositionOutOfRange { x });
        }
        for gain in [a, b] {
            if !(0.0..=1.0).contains(&gain) {
                return Err(VirtualDjError::GainOutOfRange { value: gain });
            }
        }
        Ok(CrossfaderPoint::new(x, a, b))
    }

    fn malformed(&self, reason: &'static str) -> VirtualDjError {
        let rest = &self.input[self.pos.min(self.input.len())..];
        let mut near: String = rest.chars().take(ERROR_SNIPPET_CHARS).collect();
        if rest.chars().count() > ERROR_SNIPPET_CHARS {
            near.push('…');
        }
        VirtualDjError::MalformedPoint { near, reason }
    }
}
