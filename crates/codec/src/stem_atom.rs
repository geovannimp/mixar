//! NI STEM atom JSON types (`.stem.mp4` metadata).

use serde::{Deserialize, Serialize};

/// One stem slot in the STEM atom (display name + color).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StemSlot {
    pub name: String,
    pub color: String,
}

/// Disabled-by-default mastering DSP block written into the STEM atom.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MasteringDspCompressor {
    pub enabled: bool,
    pub ratio: i32,
    pub output_gain: i32,
    pub release: f32,
    pub attack: f32,
    pub input_gain: i32,
    pub threshold: i32,
    pub hp_cutoff: i32,
    pub dry_wet: i32,
}

/// Limiter section of [`MasteringDsp`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MasteringDspLimiter {
    pub enabled: bool,
    pub release: f32,
    pub threshold: i32,
    pub ceiling: i32,
}

/// Mastering DSP metadata (not applied during playback).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MasteringDsp {
    pub compressor: MasteringDspCompressor,
    pub limiter: MasteringDspLimiter,
}

impl Default for MasteringDsp {
    fn default() -> Self {
        Self {
            compressor: MasteringDspCompressor {
                enabled: false,
                ratio: 10,
                output_gain: 0,
                release: 1.0,
                attack: 0.0001,
                input_gain: 0,
                threshold: 0,
                hp_cutoff: 20,
                dry_wet: 100,
            },
            limiter: MasteringDspLimiter {
                enabled: false,
                release: 1.0,
                threshold: 0,
                ceiling: 0,
            },
        }
    }
}

/// STEM atom payload (`version`, four stems, disabled mastering DSP).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StemAtom {
    pub version: i32,
    pub stems: [StemSlot; 4],
    pub mastering_dsp: MasteringDsp,
}

impl StemAtom {
    /// Default NI/stemgen labels and colors: drums, bass, other, vocals.
    pub fn default_ni() -> Self {
        Self {
            version: 1,
            stems: [
                StemSlot {
                    name: "Drums".to_owned(),
                    color: "#009E73".to_owned(),
                },
                StemSlot {
                    name: "Bass".to_owned(),
                    color: "#D55E00".to_owned(),
                },
                StemSlot {
                    name: "Other".to_owned(),
                    color: "#CC79A7".to_owned(),
                },
                StemSlot {
                    name: "Vocals".to_owned(),
                    color: "#56B4E9".to_owned(),
                },
            ],
            mastering_dsp: MasteringDsp::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::StemAtom;

    #[test]
    fn stem_atom_default_matches_stemgen() {
        let json = serde_json::to_string(&StemAtom::default_ni()).unwrap();
        assert!(json.contains("\"name\":\"Drums\""));
        assert!(json.contains("#009E73"));
        assert!(json.contains("\"version\":1"));
        let back: StemAtom = serde_json::from_str(&json).unwrap();
        assert_eq!(back, StemAtom::default_ni());
    }
}
