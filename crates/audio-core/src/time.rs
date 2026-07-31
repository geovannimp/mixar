//! Media time conversions (seconds ↔ milliseconds).

/// Convert floating seconds to integer milliseconds (rounded).
pub fn secs_to_ms(secs: f64) -> i32 {
    if !secs.is_finite() {
        return 0;
    }
    (secs * 1000.0).round() as i32
}

/// Convert integer milliseconds to floating seconds.
pub fn ms_to_secs(ms: i32) -> f64 {
    f64::from(ms) / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secs_to_ms_rounds_and_negatives() {
        assert_eq!(secs_to_ms(12.5), 12_500);
        assert_eq!(secs_to_ms(-0.5), -500);
        assert_eq!(secs_to_ms(0.0004), 0);
        assert_eq!(secs_to_ms(0.0006), 1);
    }

    #[test]
    fn ms_to_secs_roundtrip_center() {
        assert!((ms_to_secs(12_500) - 12.5).abs() < f64::EPSILON);
        assert!((ms_to_secs(-500) + 0.5).abs() < f64::EPSILON);
    }
}
