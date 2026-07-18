use stratum_dsp::Key;

/// Map stratum-dsp key to compact musical notation for library storage.
pub fn musical_key_from_stratum(key: &Key) -> String {
    key.name()
}

#[cfg(test)]
mod tests {
    use super::*;
    use stratum_dsp::Key;

    #[test]
    fn uses_stratum_musical_name() {
        assert_eq!(musical_key_from_stratum(&Key::Major(0)), "C");
        assert_eq!(musical_key_from_stratum(&Key::Minor(9)), "Am");
    }
}
