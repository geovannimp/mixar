//! Curve editor import/export bridge.
//!
//! The editor keeps its library in [`bus_api::CurveLibrarySpec`] form so it can
//! persist through settings unchanged. VirtualDJ stores a single A/B curve as a
//! `crossfaderCurve` name plus an optional `crossfaderCustom` point script;
//! [`import_virtualdj_curve`] turns that pair into a native editable group and
//! [`export_virtualdj_curve`] writes a group back through `config-adapters`.

use bus_api::CurveGroupSpec;
use config_adapters::virtualdj::{export_crossfader, import_crossfader, VirtualDjCrossfaderConfig};
use curve_core::CurveGroupRef;
use engine_core::{group_from_spec, group_to_spec};

/// A VirtualDJ curve imported as a native editable group.
#[derive(Clone, Debug)]
pub struct VirtualDjCurveImport {
    /// Two-channel native group (deck A is channel 0, deck B channel 1).
    pub group: CurveGroupSpec,
    /// Display name suggested by the imported curve or point script.
    pub suggested_name: String,
    /// The raw `crossfaderCurve` value that was imported.
    pub crossfader_curve: String,
    /// `crossfaderHamster` from the same VirtualDJ settings.
    pub hamster: bool,
    /// `crossfaderDisable` (or a disabled curve name) from the same settings.
    pub disabled: bool,
}

/// A native curve written back as VirtualDJ options.
#[derive(Clone, Debug)]
pub struct VirtualDjCurveExport {
    /// `crossfaderCurve` value (`Custom` for an editable native group).
    pub crossfader_curve: String,
    /// `crossfaderCustom` point script (`x=[a,b]/…`).
    pub crossfader_custom: String,
}

/// Import VirtualDJ crossfader options as an editable native curve group.
///
/// `crossfader_curve` accepts the same names and amounts as VirtualDJ (for
/// example `Smooth`, `Cut`, `30%`, or `Custom`); `crossfader_custom` is used
/// when the name is empty or `Custom`. The returned group always has two
/// channels and every segment is editable in the curve editor.
pub fn import_virtualdj_curve(
    crossfader_curve: String,
    crossfader_custom: String,
) -> Result<VirtualDjCurveImport, String> {
    let config = VirtualDjCrossfaderConfig {
        crossfader_curve: crossfader_curve.clone(),
        crossfader_custom,
        crossfader_hamster: false,
        crossfader_disable: false,
    };
    let native = import_crossfader(&config).map_err(|error| error.to_string())?;
    let suggested_name = suggested_name(&config);
    Ok(VirtualDjCurveImport {
        group: group_to_spec("", &suggested_name, &native.group),
        suggested_name,
        crossfader_curve: crossfader_curve.trim().to_string(),
        hamster: native.behavior.hamster,
        disabled: native.behavior.disabled,
    })
}

/// Export a native curve group as a VirtualDJ `crossfaderCustom` script.
///
/// Non-linear segments are flattened at the union of both channels'
/// breakpoints, matching the settings migration's documented lossiness.
pub fn export_virtualdj_curve(group: CurveGroupSpec) -> Result<VirtualDjCurveExport, String> {
    let id = group.id.clone();
    let named = group_from_spec(&group)?;
    let curve_group = named.group;
    // The id is only used to recognize a named preset; editor groups are
    // custom, so a custom reference always exports its sampled point script.
    let reference = CurveGroupRef::custom(if id.is_empty() {
        "exported_curve"
    } else {
        id.as_str()
    });
    let exported = export_crossfader(&config_adapters::virtualdj::CrossfaderConfig {
        group: curve_group,
        group_ref: reference,
        behavior: audio_core::CrossfaderCurveBehavior::NORMAL,
    })
    .map_err(|error| error.to_string())?;
    Ok(VirtualDjCurveExport {
        crossfader_curve: exported.crossfader_curve,
        crossfader_custom: exported.crossfader_custom,
    })
}

/// Display name for an imported VirtualDJ curve.
fn suggested_name(config: &VirtualDjCrossfaderConfig) -> String {
    let curve = config.crossfader_curve.trim().trim_matches('"');
    if curve.is_empty() {
        if config.crossfader_custom.trim().is_empty() {
            "VirtualDJ smooth".to_string()
        } else {
            "VirtualDJ custom".to_string()
        }
    } else if curve.eq_ignore_ascii_case("custom") {
        "VirtualDJ custom".to_string()
    } else {
        format!("VirtualDJ {curve}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bus_api::CurveSegmentKind;

    #[test]
    fn imports_a_custom_script_as_an_editable_two_channel_group() {
        let imported =
            import_virtualdj_curve("Custom".into(), "0=[1,0]/0.5=[0.25,0.75]/1=[0,1]".into())
                .expect("script imports");

        assert_eq!(imported.suggested_name, "VirtualDJ custom");
        assert!(!imported.hamster);
        assert!(!imported.disabled);
        assert_eq!(imported.group.curves.len(), 2);
        assert_eq!(imported.group.curves[0].points.len(), 3);
        assert_eq!(imported.group.curves[0].points[1].x, 0.5);
        assert_eq!(imported.group.curves[0].points[1].y, 0.25);
        assert_eq!(imported.group.curves[1].points[1].y, 0.75);
        assert!(imported.group.curves[0]
            .segments
            .iter()
            .all(|segment| segment.kind == CurveSegmentKind::Linear));
    }

    #[test]
    fn imports_named_presets_as_native_curves() {
        let imported = import_virtualdj_curve("Cut".into(), String::new()).expect("preset imports");
        assert_eq!(imported.suggested_name, "VirtualDJ Cut");
        assert_eq!(imported.group.curves.len(), 2);
        assert_eq!(imported.crossfader_curve, "Cut");
        // The 15% cut has breakpoints at 0.15/0.85, so the native curve keeps
        // the same piecewise-linear shape and stays editable.
        assert!(imported.group.curves[0]
            .points
            .iter()
            .any(|point| (point.x - 0.15).abs() < 1e-6));
    }

    #[test]
    fn rejects_a_malformed_script() {
        let error = import_virtualdj_curve("Custom".into(), "0=[1,0]/oops".into())
            .expect_err("malformed script is rejected");
        assert!(error.contains("malformed"), "{error}");
    }

    #[test]
    fn exports_a_native_group_as_a_custom_script() {
        let mut group = import_virtualdj_curve("Custom".into(), "0=[1,0]/1=[0,1]".into())
            .expect("script imports")
            .group;
        group.id = "tight".into();
        group.display_name = "Tight".into();

        let exported = export_virtualdj_curve(group).expect("group exports");
        assert_eq!(exported.crossfader_curve, "Custom");
        assert_eq!(exported.crossfader_custom, "0=[1,0]/1=[0,1]");
    }

    #[test]
    fn export_rejects_a_malformed_group() {
        let mut group = import_virtualdj_curve("Custom".into(), "0=[1,0]/1=[0,1]".into())
            .expect("script imports")
            .group;
        group.id = "broken".into();
        group.curves[0].points.remove(1);
        let error = export_virtualdj_curve(group).expect_err("invalid group is rejected");
        assert!(!error.is_empty());
    }
}
