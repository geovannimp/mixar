//! Star-rating scales for lofty's generic `Popularimeter`.
//!
//! No tag format standardises the integer behind a star rating, so lofty maps
//! popularimeters through a [`RatingProvider`] selected by the POPM email. Its
//! built-in providers cover MusicBee, Windows Media Player and MusicBrainz
//! Picard. Mixar registers the fallback for every other email
//! (`traktor@native-instruments.de`, `Serato`, an empty POPM email, unknown
//! apps) and for MP4's email-less `rate` atom.
//!
//! The fallback **writes** the stepped `1/64/128/196/255` set and **reads** on
//! the linear `round(star * 51)` thresholds. The mismatch is deliberate: lofty
//! converts a `POPM` frame to text and back, so the raw byte of a frame Mixar
//! does not own cannot survive a save — re-emitting the stepped value keeps a
//! stepped writer's (MusicBee, Windows Media Player, Serato, …) frame
//! byte-identical instead of shifting its star, while the linear read
//! thresholds land both sets on the same star (1→1, 64→2, 128→3, 196→4,
//! 255→5). MP4 `rate` re-uses the star count itself (1–5, what Mixar writes)
//! and falls back to the 0–100 percentage scale some apps write.
//!
//! Three lossy edges are inherent to lofty's model and documented here rather
//! than worked around: `StarRating` has no "unrated" variant, so a stored `0`
//! reads as one star; a provider cannot see the email inside `rate` or
//! `convert_raw`, so a foreign frame written on another scale is re-encoded
//! star-for-star rather than byte-for-byte; and lofty's `Popularimeter` text
//! form is `email|stars|counter` with an unescaped email, so a (pathological)
//! POPM email containing `|` fails to parse on the way back out and its frame
//! is dropped by the next write.
//!
//! [`set_custom_rating_provider`] is process-global and panics when called
//! twice, so [`install`] runs it under a [`Once`].

use std::sync::Once;

use lofty::tag::items::popularimeter::{set_custom_rating_provider, RatingProvider, StarRating};
use lofty::tag::TagType;

/// POPM emails Mixar mirrors, in the order they are written and preferred on
/// read. MusicBee and Windows Media Player use `1/64/128/196/255`; Picard's
/// frame goes through lofty's provider (`round(star * 51)`); Traktor's email is
/// unknown to lofty, so it takes the fallback's stepped bytes.
pub(crate) const POPM_EMAILS: [&str; 4] = [
    "traktor@native-instruments.de",
    "users@musicbrainz.org",
    "MusicBee",
    "Windows Media Player 9 Series",
];

/// POPM bytes for 1–5 stars on the stepped `1/64/128/196/255` scale, indexed by
/// star count (`0` is "unrated" and never written).
const STEPPED_BYTES: [u8; 6] = [0, 1, 64, 128, 196, 255];

/// Install Mixar's fallback popularimeter mapping, once per process.
/// Called on every tag read and write; the [`Once`] keeps lofty's
/// panic-on-second-install from firing while other code (tests included)
/// reaches the same path concurrently.
pub(crate) fn install() {
    static INSTALL: Once = Once::new();
    INSTALL.call_once(|| set_custom_rating_provider(MixarRatingProvider));
}

/// The fallback mapping for popularimeters lofty's built-in providers do not
/// claim.
struct MixarRatingProvider;

impl RatingProvider for MixarRatingProvider {
    fn supports_email(&self, _: &str) -> bool {
        true
    }

    fn rate(&self, tag_type: TagType, rating: StarRating) -> u8 {
        let stars = rating as u8;
        match tag_type {
            TagType::Mp4Ilst => stars,
            TagType::VorbisComments => stars.saturating_mul(20),
            // Stepped (ID3v2, plus any format lofty adds later), so a foreign
            // stepped writer's frame keeps its exact byte across a save.
            _ => STEPPED_BYTES[usize::from(stars)],
        }
    }

    fn convert_raw(&self, tag_type: TagType, rating: u8) -> StarRating {
        match tag_type {
            TagType::Mp4Ilst => mp4_stars(rating),
            TagType::VorbisComments => percent_stars(rating),
            _ => linear_stars(rating),
        }
    }
}

/// A 1–5 count as a [`StarRating`]; `0` clamps to one star (lofty has no
/// "unrated" variant).
fn star(count: u8) -> StarRating {
    match count {
        0 | 1 => StarRating::One,
        2 => StarRating::Two,
        3 => StarRating::Three,
        4 => StarRating::Four,
        _ => StarRating::Five,
    }
}

/// Linear `round(star * 51)` POPM byte scale as a 1–5 star count (`204` = four
/// stars). The thresholds also land on the right star for the stepped
/// `1/64/128/196/255` writers.
pub(crate) fn linear_stars_count(value: u8) -> u8 {
    #[allow(clippy::match_overlapping_arm)]
    match value {
        ..=51 => 1,
        ..=102 => 2,
        ..=153 => 3,
        ..=204 => 4,
        _ => 5,
    }
}

/// Linear `round(star * 51)` POPM byte scale (`204` = four stars).
fn linear_stars(value: u8) -> StarRating {
    star(linear_stars_count(value))
}

/// 0–100 percentage scale (`80` = four stars), rounded to the nearest star so
/// it agrees with the reader's text parsing (`75` is four, not three).
fn percent_stars(value: u8) -> StarRating {
    star(((u16::from(value) + 10) / 20) as u8)
}

/// MP4 `rate`: the star count itself (`3` = three stars), or a 0–100
/// percentage for writers that store the normalized scale.
fn mp4_stars(value: u8) -> StarRating {
    if value <= 5 {
        star(value)
    } else {
        percent_stars(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_scale_maps_canonical_writer_steps_onto_stars() {
        // Traktor / Picard (`round(star * 51)`).
        for (value, expected) in [
            (51, StarRating::One),
            (102, StarRating::Two),
            (153, StarRating::Three),
            (204, StarRating::Four),
            (255, StarRating::Five),
        ] {
            assert_eq!(linear_stars(value), expected, "linear {value}");
        }
        // MusicBee / Windows Media Player / Serato (`1/64/128/196/255`) land on
        // the same stars.
        for (value, expected) in [
            (1, StarRating::One),
            (64, StarRating::Two),
            (128, StarRating::Three),
            (196, StarRating::Four),
            (255, StarRating::Five),
        ] {
            assert_eq!(linear_stars(value), expected, "stepped {value}");
        }
        assert_eq!(linear_stars(0), StarRating::One, "0 clamps to one star");
    }

    #[test]
    fn mp4_rate_accepts_stars_and_percentages() {
        for (value, expected) in [
            (1, StarRating::One),
            (4, StarRating::Four),
            (5, StarRating::Five),
            // Percentage scale: 80 means four stars.
            (20, StarRating::One),
            (80, StarRating::Four),
            (100, StarRating::Five),
            // Between the two scales 5 is still the star count.
            (6, StarRating::One),
        ] {
            assert_eq!(mp4_stars(value), expected, "rate {value}");
        }
    }

    #[test]
    fn fallback_writes_the_stepped_set_so_foreign_frames_keep_their_byte() {
        let provider = MixarRatingProvider;
        for (count, byte) in [(1, 1), (2, 64), (3, 128), (4, 196), (5, 255)] {
            assert_eq!(provider.rate(TagType::Id3v2, star(count)), byte);
            // A stepped writer's frame is a byte-exact fixed point across a save.
            assert_eq!(
                provider.rate(TagType::Id3v2, provider.convert_raw(TagType::Id3v2, byte)),
                byte,
                "stepped byte {byte} must survive a rewrite"
            );
        }
        // A 51-scale foreign byte is re-encoded on the stepped set but keeps its
        // star count (the read thresholds agree with both scales).
        for byte in [51_u8, 102, 153, 204, 255] {
            let stars = provider.convert_raw(TagType::Id3v2, byte);
            assert_eq!(
                provider.convert_raw(TagType::Id3v2, provider.rate(TagType::Id3v2, stars)),
                stars,
                "51-scale byte {byte} keeps its star"
            );
        }
    }

    #[test]
    fn provider_scales_round_trip_against_their_own_writer() {
        let provider = MixarRatingProvider;
        for tag_type in [TagType::Id3v2, TagType::VorbisComments, TagType::Mp4Ilst] {
            for count in 1..=5 {
                let star_rating = star(count);
                let raw = provider.rate(tag_type, star_rating);
                assert_eq!(
                    provider.convert_raw(tag_type, raw),
                    star_rating,
                    "{tag_type:?} star {count} -> {raw}"
                );
            }
        }
    }

    #[test]
    fn percent_scale_rounds_to_the_nearest_star() {
        assert_eq!(percent_stars(0), StarRating::One);
        assert_eq!(percent_stars(19), StarRating::One);
        assert_eq!(percent_stars(21), StarRating::One);
        assert_eq!(percent_stars(60), StarRating::Three);
        // Nearest star, matching the reader's text parsing (`75 / 20` is 3.75).
        assert_eq!(percent_stars(75), StarRating::Four);
        assert_eq!(percent_stars(99), StarRating::Five);
    }
}
