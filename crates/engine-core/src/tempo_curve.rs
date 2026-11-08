//! Tempo-fader curve resolution and installation.
//!
//! The DSP deck owns the compiled [`engine_dsp::TempoCurve`]: one scalar curve
//! bound to a signed `±tempo_range` pitch axis. This module is the control-thread
//! layer that turns a settings/controller id into that compiled curve. It owns
//! no curve math: shapes come from `curve-core`, and custom groups come from the
//! shared curve library.
//!
//! Resolution order for a selection id:
//!
//! 1. the `linear` tempo default (the historical fader mapping),
//! 2. a named one-channel custom group in the library.
//!
//! A multi-channel group is rejected for this scalar target. Unknown ids fall
//! back to `linear` so playback never fails to start or select.

use anyhow::Result;
use bus_api::DEFAULT_TEMPO_CURVE_ID;
use curve_core::{CurveLibrary, ResolvedCurveGroup};

/// A tempo fader drives exactly one curve output.
pub const TEMPO_CURVE_CHANNELS: usize = engine_dsp::TEMPO_CURVE_CHANNELS;

/// The active tempo-curve selection as the control thread remembers it.
#[derive(Clone, Debug, PartialEq)]
pub struct TempoCurveSelection {
    /// Canonical id of the active `linear` default or custom group.
    pub id: String,
    /// Human-readable name for settings and logs.
    pub display_name: String,
}

impl Default for TempoCurveSelection {
    fn default() -> Self {
        Self::linear()
    }
}

impl TempoCurveSelection {
    /// The identity selection used when none is configured.
    pub fn linear() -> Self {
        Self {
            id: DEFAULT_TEMPO_CURVE_ID.to_string(),
            display_name: "Linear".to_string(),
        }
    }
}

/// A compiled tempo curve plus the selection it came from.
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedTempoCurve {
    pub curve: ResolvedCurveGroup,
    pub selection: TempoCurveSelection,
}

/// The default linear tempo curve.
pub fn linear_tempo_curve() -> ResolvedTempoCurve {
    ResolvedTempoCurve {
        curve: engine_dsp::linear_tempo_curve(),
        selection: TempoCurveSelection::linear(),
    }
}

/// Resolve a preset id or custom group id to a compiled tempo curve.
///
/// The returned curve always has [`TEMPO_CURVE_CHANNELS`] outputs.
pub fn resolve_tempo_curve(id: &str, library: &CurveLibrary) -> Result<ResolvedTempoCurve, String> {
    let id = id.trim();
    if id.is_empty() {
        return Err("tempo curve id is empty".to_string());
    }
    if id.eq_ignore_ascii_case(DEFAULT_TEMPO_CURVE_ID) {
        return Ok(linear_tempo_curve());
    }

    let group = library
        .group(id)
        .ok_or_else(|| format!("unknown tempo curve `{id}`"))?;
    group
        .group
        .expect_channels(TEMPO_CURVE_CHANNELS)
        .map_err(|error| format!("curve group `{id}`: {error}"))?;
    let curve = ResolvedCurveGroup::single(&group.group)
        .map_err(|error| format!("curve group `{id}`: {error}"))?;
    let display_name = if group.display_name.is_empty() {
        group.id.clone()
    } else {
        group.display_name.clone()
    };
    Ok(ResolvedTempoCurve {
        curve,
        selection: TempoCurveSelection {
            id: group.id.clone(),
            display_name,
        },
    })
}

impl crate::Engine {
    /// Install an already-compiled tempo curve on one deck.
    ///
    /// The deck keeps its range and fader position; a synced deck remaps its
    /// displayed position against the new curve. Resolve and compile before
    /// calling; this only swaps the prepared curve under the DSP lock.
    pub fn set_tempo_curve(&mut self, deck_id: usize, curve: ResolvedCurveGroup) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().expect("DSP engine mutex poisoned");
        let deck = dsp
            .deck_mut(deck_id)
            .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {deck_id}"))?;
        deck.set_tempo_curve(curve)?;
        Ok(())
    }

    /// Select a tempo curve by id for one deck.
    ///
    /// Returns `Some(message)` when the id did not resolve and `linear` was
    /// installed instead; callers publish that as a notice. Playback never fails
    /// because of a stale id.
    pub fn select_tempo_curve(&mut self, deck_id: usize, id: &str) -> Result<Option<String>> {
        let resolved = match resolve_tempo_curve(id, &self.curve_library) {
            Ok(resolved) => resolved,
            Err(error) => {
                let fallback = linear_tempo_curve();
                self.set_tempo_curve(deck_id, fallback.curve)?;
                self.set_tempo_curve_selection(deck_id, fallback.selection);
                return Ok(Some(format!("{error}; using {DEFAULT_TEMPO_CURVE_ID}")));
            }
        };
        self.set_tempo_curve(deck_id, resolved.curve)?;
        self.set_tempo_curve_selection(deck_id, resolved.selection);
        Ok(None)
    }

    /// The active tempo-curve selection id and display name for a deck.
    pub fn tempo_curve_selection(&self, deck_id: usize) -> Option<&TempoCurveSelection> {
        self.tempo_curve_selections.get(deck_id)
    }

    fn set_tempo_curve_selection(&mut self, deck_id: usize, selection: TempoCurveSelection) {
        if let Some(slot) = self.tempo_curve_selections.get_mut(deck_id) {
            *slot = selection;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bus_api::{
        CurveGroupSpec, CurveLibrarySpec, CurvePointSpec, CurveSegmentKind, CurveSegmentSpec,
        CurveSpecSpec,
    };

    fn linear_spec(y0: f32, y1: f32) -> CurveSpecSpec {
        CurveSpecSpec {
            points: vec![
                CurvePointSpec { x: 0.0, y: y0 },
                CurvePointSpec { x: 1.0, y: y1 },
            ],
            segments: vec![CurveSegmentSpec {
                kind: CurveSegmentKind::Linear,
                rate: 0.0,
                c1x: 0.0,
                c1y: 0.0,
                c2x: 0.0,
                c2y: 0.0,
            }],
        }
    }

    fn library_with(group_id: &str, channels: usize) -> CurveLibrary {
        let curves = (0..channels)
            .map(|_| linear_spec(0.0, 1.0))
            .collect::<Vec<_>>();
        let spec = CurveLibrarySpec {
            groups: vec![CurveGroupSpec {
                id: group_id.to_string(),
                display_name: "Custom Tempo".to_string(),
                curves,
            }],
            sets: vec![],
        };
        crate::curve_library::library_from_spec(&spec).expect("valid library")
    }

    #[test]
    fn default_linear_id_resolves_without_a_custom_group() {
        let resolved = resolve_tempo_curve("linear", &CurveLibrary::default()).expect("linear");
        assert_eq!(resolved.curve.channels(), TEMPO_CURVE_CHANNELS);
        assert_eq!(resolved.selection, TempoCurveSelection::linear());
        assert_eq!(resolved.selection.id, DEFAULT_TEMPO_CURVE_ID);
    }

    #[test]
    fn custom_one_channel_group_resolves_and_rejects_channel_counts() {
        let library = library_with("taper", 1);
        let resolved = resolve_tempo_curve("taper", &library).expect("custom group");
        assert_eq!(resolved.selection.id, "taper");
        assert_eq!(resolved.selection.display_name, "Custom Tempo");

        let two_channel = library_with("pair", 2);
        let error = resolve_tempo_curve("pair", &two_channel).unwrap_err();
        assert!(error.contains("needs 1 curves"), "{error}");
    }

    #[test]
    fn unknown_id_is_an_error() {
        let error = resolve_tempo_curve("nope", &CurveLibrary::default()).unwrap_err();
        assert!(error.contains("unknown tempo curve"), "{error}");
    }
}
