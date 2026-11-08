//! Channel-fader curve resolution, selection, and installation.
//!
//! The DSP strip stores one compiled [`ResolvedCurveGroup`] per channel and
//! evaluates it once per block. This module is the control-thread layer that
//! turns a settings/controller id into those tables. It owns no curve math: the
//! shapes come from `curve-core`, and custom groups come from the shared curve
//! library.
//!
//! Resolution order for a selection id:
//!
//! 1. the `linear` identity preset,
//! 2. a named one-channel custom group in the library.
//!
//! A multi-channel group is rejected for this scalar target. Unknown ids fall
//! back to `linear` so playback never fails to start or select.

use anyhow::Result;
use bus_api::DEFAULT_CHANNEL_FADER_CURVE_ID;
use curve_core::{CurveLibrary, ResolvedCurveGroup};

/// A channel fader drives exactly one curve output.
pub const CHANNEL_FADER_CHANNELS: usize = engine_dsp::CHANNEL_FADER_CHANNELS;

/// The active channel-fader selection as the control thread remembers it.
#[derive(Clone, Debug, PartialEq)]
pub struct ChannelFaderCurveSelection {
    /// Canonical id of the active `linear` preset or custom group.
    pub id: String,
    /// Human-readable name for settings and logs.
    pub display_name: String,
}

impl Default for ChannelFaderCurveSelection {
    fn default() -> Self {
        Self::linear()
    }
}

impl ChannelFaderCurveSelection {
    /// The identity selection used when none is configured.
    pub fn linear() -> Self {
        Self {
            id: DEFAULT_CHANNEL_FADER_CURVE_ID.to_string(),
            display_name: "Linear".to_string(),
        }
    }
}

/// A compiled channel-fader curve plus the selection it came from.
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedChannelFaderCurve {
    pub curve: ResolvedCurveGroup,
    pub selection: ChannelFaderCurveSelection,
}

/// The default identity channel-fader curve.
pub fn linear_channel_fader_curve() -> ResolvedChannelFaderCurve {
    ResolvedChannelFaderCurve {
        curve: engine_dsp::linear_fader_curve(),
        selection: ChannelFaderCurveSelection::linear(),
    }
}

/// Resolve a preset id or custom group id to compiled channel-fader tables.
///
/// The returned curve always has [`CHANNEL_FADER_CHANNELS`] outputs.
pub fn resolve_channel_fader_curve(
    id: &str,
    library: &CurveLibrary,
) -> Result<ResolvedChannelFaderCurve, String> {
    let id = id.trim();
    if id.is_empty() {
        return Err("channel-fader curve id is empty".to_string());
    }
    if id.eq_ignore_ascii_case(DEFAULT_CHANNEL_FADER_CURVE_ID) {
        return Ok(linear_channel_fader_curve());
    }

    let group = library
        .group(id)
        .ok_or_else(|| format!("unknown channel-fader curve `{id}`"))?;
    group
        .group
        .expect_channels(CHANNEL_FADER_CHANNELS)
        .map_err(|error| format!("curve group `{id}`: {error}"))?;
    let curve = ResolvedCurveGroup::single(&group.group)
        .map_err(|error| format!("curve group `{id}`: {error}"))?;
    let display_name = if group.display_name.is_empty() {
        group.id.clone()
    } else {
        group.display_name.clone()
    };
    Ok(ResolvedChannelFaderCurve {
        curve,
        selection: ChannelFaderCurveSelection {
            id: group.id.clone(),
            display_name,
        },
    })
}

impl crate::Engine {
    /// Install an already-compiled scalar fader curve on every channel.
    ///
    /// Resolve and compile before calling; this only swaps the prepared tables
    /// under the DSP lock, so a running session changes without a restart.
    pub fn set_channel_fader_curve(&mut self, curve: ResolvedCurveGroup) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().expect("DSP engine mutex poisoned");
        dsp.mixer_mut().set_channel_fader_curve(curve)?;
        Ok(())
    }

    /// Select a preset or custom one-channel group by id.
    ///
    /// Returns `Some(message)` when the id did not resolve and `linear` was
    /// installed instead; callers publish that as a notice. Playback never fails
    /// because of a stale id.
    pub fn select_channel_fader_curve(&mut self, id: &str) -> Result<Option<String>> {
        let resolved = match resolve_channel_fader_curve(id, &self.curve_library) {
            Ok(resolved) => resolved,
            Err(error) => {
                let fallback = linear_channel_fader_curve();
                self.set_channel_fader_curve(fallback.curve)?;
                self.channel_fader_selection = fallback.selection;
                return Ok(Some(format!(
                    "{error}; using {DEFAULT_CHANNEL_FADER_CURVE_ID}"
                )));
            }
        };
        self.set_channel_fader_curve(resolved.curve)?;
        self.channel_fader_selection = resolved.selection;
        Ok(None)
    }

    /// The active channel-fader selection id and display name.
    pub fn channel_fader_curve_selection(&self) -> &ChannelFaderCurveSelection {
        &self.channel_fader_selection
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
                display_name: "Custom Fader".to_string(),
                curves,
            }],
            sets: vec![],
        };
        crate::curve_library::library_from_spec(&spec).expect("valid library")
    }

    #[test]
    fn default_linear_id_resolves_without_a_custom_group() {
        let resolved =
            resolve_channel_fader_curve("linear", &CurveLibrary::default()).expect("linear");
        assert_eq!(resolved.curve.channels(), CHANNEL_FADER_CHANNELS);
        assert_eq!(resolved.selection, ChannelFaderCurveSelection::linear());
        assert!((resolved.curve.evaluate(0.25).unwrap()[0] - 0.25).abs() < 1e-5);
    }

    #[test]
    fn custom_one_channel_group_resolves_and_rejects_channel_counts() {
        let library = library_with("taper", 1);
        let resolved = resolve_channel_fader_curve("taper", &library).expect("custom group");
        assert_eq!(resolved.selection.id, "taper");
        assert_eq!(resolved.selection.display_name, "Custom Fader");

        let two_channel = library_with("pair", 2);
        let error = resolve_channel_fader_curve("pair", &two_channel).unwrap_err();
        assert!(error.contains("needs 1 curves"), "{error}");
    }

    #[test]
    fn unknown_id_is_an_error() {
        let error = resolve_channel_fader_curve("nope", &CurveLibrary::default()).unwrap_err();
        assert!(error.contains("unknown channel-fader curve"), "{error}");
    }
}
