use std::collections::BTreeMap;

use audio_core::CrossfaderCurveBehavior;
use curve_core::{
    CurveError, CurveGroup, CurveGroupRef, CurvePoint, CurveRefKind, CurveSpec, SegmentKind,
};

use super::*;

/// Tolerance for sampled gains and fitted curves.
const EPSILON: f32 = 1e-5;

fn options(curve: &str, custom: &str) -> VirtualDjCrossfaderConfig {
    VirtualDjCrossfaderConfig {
        crossfader_curve: curve.to_string(),
        crossfader_custom: custom.to_string(),
        crossfader_hamster: false,
        crossfader_disable: false,
    }
}

fn linear_curve(points: &[(f32, f32)]) -> CurveSpec {
    CurveSpec::new(
        points.iter().map(|&(x, y)| CurvePoint::new(x, y)).collect(),
        vec![SegmentKind::Linear; points.len() - 1],
    )
    .expect("test points are valid")
}

fn gain(group: &CurveGroup, channel: usize, x: f32) -> f32 {
    group
        .evaluate_channel(channel, x)
        .expect("the group has both channels")
}

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= EPSILON,
        "expected {expected}, got {actual}"
    );
}

fn positions(points: &[CrossfaderPoint]) -> Vec<f32> {
    points.iter().map(|point| point.x).collect()
}

// --- point parsing -------------------------------------------------------

#[test]
fn parses_slash_separated_points() {
    let points = parse_points("0=[1,0] / 0.5=[0.5,0.5] / 1=[0,1]").expect("valid script");
    assert_eq!(
        points,
        vec![
            CrossfaderPoint::new(0.0, 1.0, 0.0),
            CrossfaderPoint::new(0.5, 0.5, 0.5),
            CrossfaderPoint::new(1.0, 0.0, 1.0),
        ]
    );
}

#[test]
fn parses_concatenated_points() {
    let points = parse_points("0=[1,0]0.5=[0.5,0.5]1=[0,1]").expect("valid script");
    assert_eq!(
        points,
        vec![
            CrossfaderPoint::new(0.0, 1.0, 0.0),
            CrossfaderPoint::new(0.5, 0.5, 0.5),
            CrossfaderPoint::new(1.0, 0.0, 1.0),
        ]
    );
}

#[test]
fn parses_whitespace_anywhere() {
    let points = parse_points(" 0 = [ 1 , 0 ] 1 = [ 0 , 1 ] ").expect("valid script");
    assert_eq!(
        points,
        vec![
            CrossfaderPoint::new(0.0, 1.0, 0.0),
            CrossfaderPoint::new(1.0, 0.0, 1.0),
        ]
    );
}

#[test]
fn formats_points_that_parse_back_identically() {
    let points = vec![
        CrossfaderPoint::new(0.0, 1.0, 0.0),
        CrossfaderPoint::new(0.25, 0.8, 0.1),
        CrossfaderPoint::new(1.0, 0.0, 1.0),
    ];
    let script = format_points(&points);
    assert_eq!(script, "0=[1,0]/0.25=[0.8,0.1]/1=[0,1]");
    assert_eq!(parse_points(&script).expect("valid output"), points);
}

#[test]
fn rejects_unclosed_point() {
    assert!(matches!(
        parse_points("0=[1,0]/0.5=[0.5,0.5"),
        Err(VirtualDjError::MalformedPoint { .. })
    ));
}

#[test]
fn rejects_missing_equals() {
    assert!(matches!(
        parse_points("0=[1,0]/0.5[0.5,0.5]"),
        Err(VirtualDjError::MalformedPoint { reason, .. })
            if reason.contains("`=`")
    ));
}

#[test]
fn rejects_non_numeric_position() {
    assert!(matches!(
        parse_points("zero=[1,0]"),
        Err(VirtualDjError::MalformedPoint { .. })
    ));
}

#[test]
fn rejects_position_out_of_range() {
    assert!(matches!(
        parse_points("0=[1,0]/1.5=[0,1]"),
        Err(VirtualDjError::PositionOutOfRange { .. })
    ));
}

#[test]
fn rejects_gain_out_of_range() {
    assert!(matches!(
        parse_points("0=[1,0]/1=[0,2]"),
        Err(VirtualDjError::GainOutOfRange { value: 2.0 })
    ));
}

#[test]
fn rejects_too_few_points() {
    assert!(matches!(
        parse_points("0=[1,0]"),
        Err(VirtualDjError::TooFewPoints { count: 1 })
    ));
    assert!(matches!(
        parse_points("   "),
        Err(VirtualDjError::TooFewPoints { count: 0 })
    ));
}

#[test]
fn rejects_missing_start_and_end() {
    assert!(matches!(
        parse_points("0.25=[1,0]/1=[0,1]"),
        Err(VirtualDjError::MissingStart { .. })
    ));
    assert!(matches!(
        parse_points("0=[1,0]/0.75=[0,1]"),
        Err(VirtualDjError::MissingEnd { .. })
    ));
}

#[test]
fn rejects_non_increasing_positions() {
    assert!(matches!(
        parse_points("0=[1,0]/0.75=[0.5,0.5]/0.25=[0.5,0.5]/1=[0,1]"),
        Err(VirtualDjError::NonIncreasingPosition { .. })
    ));
}

// --- import --------------------------------------------------------------

#[test]
fn imports_smooth_as_constant_power() {
    let native = import_crossfader(&options("Smooth", "")).expect("smooth imports");
    assert_eq!(
        native.group_ref,
        CurveGroupRef::preset(CONSTANT_POWER_PRESET)
    );
    assert!(native.behavior.is_normal());
    assert_close(gain(&native.group, 0, 0.0), 1.0);
    assert_close(gain(&native.group, 0, 0.5), std::f32::consts::FRAC_1_SQRT_2);
    assert_close(gain(&native.group, 1, 0.5), std::f32::consts::FRAC_1_SQRT_2);
    assert_close(gain(&native.group, 1, 1.0), 1.0);
}

#[test]
fn imports_default_as_linear() {
    let native = import_crossfader(&options("default", "")).expect("default imports");
    assert_eq!(native.group_ref, CurveGroupRef::preset(LINEAR_PRESET));
    assert_close(gain(&native.group, 0, 0.0), 1.0);
    assert_close(gain(&native.group, 0, 0.25), 0.75);
    assert_close(gain(&native.group, 1, 0.25), 0.25);
    assert_close(gain(&native.group, 1, 1.0), 1.0);
}

#[test]
fn imports_named_fast_cuts() {
    for (name, fade) in [
        ("Cut", CUT_FADE),
        ("scratch", SCRATCH_FADE),
        ("FULL", FULL_FADE),
    ] {
        let native = import_crossfader(&options(name, "")).expect("named cut imports");
        assert_eq!(
            native.group_ref,
            CurveGroupRef::preset_with_param(FAST_CUT_PRESET, fade)
        );
        assert_close(gain(&native.group, 0, 0.0), 1.0);
        assert_close(gain(&native.group, 0, 1.0 - fade), 1.0);
        assert_close(gain(&native.group, 0, 1.0), 0.0);
        assert_close(gain(&native.group, 1, 0.0), 0.0);
        assert_close(gain(&native.group, 1, fade), 1.0);
        assert_close(gain(&native.group, 1, 1.0), 1.0);
    }
}

#[test]
fn imports_numeric_amount() {
    // VirtualDJ's amount is the fraction of travel at full gain, so the native
    // fade length is `1 - amount`.
    let native = import_crossfader(&options("0.3", "")).expect("amount imports");
    assert_eq!(
        native.group_ref,
        CurveGroupRef::preset_with_param(FAST_CUT_PRESET, 0.7)
    );
    assert_close(gain(&native.group, 0, 0.3), 1.0);
    assert_close(gain(&native.group, 0, 0.65), 0.5);
    assert_close(gain(&native.group, 1, 0.35), 0.5);
    assert_close(gain(&native.group, 1, 1.0), 1.0);
}

#[test]
fn imports_percent_amount() {
    let numeric = import_crossfader(&options("0.3", "")).expect("amount imports");
    let percent = import_crossfader(&options("30%", "")).expect("percent imports");
    assert_eq!(percent, numeric);
}

#[test]
fn imports_full_and_zero_amounts() {
    let full = import_crossfader(&options("1", "")).expect("full amount imports");
    assert_eq!(
        full.group_ref,
        CurveGroupRef::preset_with_param(FAST_CUT_PRESET, FULL_FADE)
    );

    let none = import_crossfader(&options("0", "")).expect("zero amount imports");
    assert_eq!(none.group_ref, CurveGroupRef::preset(LINEAR_PRESET));
}

#[test]
fn rejects_amount_out_of_range() {
    for value in ["1.5", "-0.1", "120%"] {
        assert!(
            matches!(
                import_crossfader(&options(value, "")),
                Err(VirtualDjError::AmountOutOfRange { .. })
            ),
            "`{value}` must be rejected"
        );
    }
}

#[test]
fn rejects_unknown_curve_name() {
    assert!(matches!(
        import_crossfader(&options("Wobble", "")),
        Err(VirtualDjError::InvalidCurve { value }) if value == "Wobble"
    ));
}

#[test]
fn imports_custom_points() {
    let native = import_crossfader(&options("Custom", "0=[1,0]/0.25=[0.5,0.5]/1=[0,1]"))
        .expect("custom curve imports");
    assert_eq!(native.group_ref, CurveGroupRef::custom(CUSTOM_GROUP_ID));
    assert_eq!(
        native.group_ref.kind,
        CurveRefKind::Custom {
            id: CUSTOM_GROUP_ID.to_string()
        }
    );
    assert_close(gain(&native.group, 0, 0.0), 1.0);
    assert_close(gain(&native.group, 0, 0.25), 0.5);
    assert_close(gain(&native.group, 1, 0.25), 0.5);
    assert_close(gain(&native.group, 1, 0.5), 0.5 + 0.25 / 0.75 * 0.5);
    assert_close(gain(&native.group, 1, 1.0), 1.0);
}

#[test]
fn custom_requires_a_point_script() {
    assert!(matches!(
        import_crossfader(&options("Custom", "  ")),
        Err(VirtualDjError::MissingCustomCurve)
    ));
}

#[test]
fn ignores_stale_custom_points_for_a_named_preset() {
    let native =
        import_crossfader(&options("Smooth", "0=[1,0]/1=[0,1]")).expect("the named preset wins");
    assert_eq!(
        native.group_ref,
        CurveGroupRef::preset(CONSTANT_POWER_PRESET)
    );
}

#[test]
fn uses_custom_points_when_the_curve_is_empty() {
    let native =
        import_crossfader(&options("", "0=[1,0]/1=[0,1]")).expect("the script selects the curve");
    assert_eq!(native.group_ref, CurveGroupRef::custom(CUSTOM_GROUP_ID));
    assert_close(gain(&native.group, 0, 0.5), 0.5);
}

#[test]
fn disabled_curve_name_sets_the_flag() {
    let native = import_crossfader(&options("Disabled", "")).expect("disabled imports");
    assert!(native.behavior.disabled);
    assert!(!native.behavior.hamster);
    assert_eq!(
        native.group_ref,
        CurveGroupRef::preset(CONSTANT_POWER_PRESET)
    );
}

#[test]
fn reads_hamster_and_disable_options() {
    let native = import_crossfader(&VirtualDjCrossfaderConfig {
        crossfader_curve: "Linear".to_string(),
        crossfader_custom: String::new(),
        crossfader_hamster: true,
        crossfader_disable: true,
    })
    .expect("flags import");
    assert_eq!(
        native.behavior,
        CrossfaderCurveBehavior {
            disabled: true,
            hamster: true,
        }
    );
}

// --- options -------------------------------------------------------------

#[test]
fn reads_all_four_options() {
    let map = BTreeMap::from([
        ("crossfaderCurve".to_string(), "\"Smooth\"".to_string()),
        ("crossfaderCustom".to_string(), String::new()),
        ("crossfaderHamster".to_string(), "yes".to_string()),
        ("crossfaderDisable".to_string(), "0".to_string()),
        ("volumeFader".to_string(), "ignored".to_string()),
    ]);
    let config = VirtualDjCrossfaderConfig::from_options(&map).expect("valid map");
    assert_eq!(
        config,
        VirtualDjCrossfaderConfig {
            crossfader_curve: "Smooth".to_string(),
            crossfader_custom: String::new(),
            crossfader_hamster: true,
            crossfader_disable: false,
        }
    );
}

#[test]
fn accepts_every_boolean_spelling() {
    for (value, expected) in [
        ("yes", true),
        ("ON", true),
        ("True", true),
        ("1", true),
        ("no", false),
        ("off", false),
        ("FALSE", false),
        ("0", false),
    ] {
        let map = BTreeMap::from([("crossfaderHamster".to_string(), value.to_string())]);
        let config = VirtualDjCrossfaderConfig::from_options(&map).expect("valid boolean");
        assert_eq!(config.crossfader_hamster, expected, "`{value}`");
    }
}

#[test]
fn rejects_invalid_boolean() {
    let map = BTreeMap::from([("crossfaderDisable".to_string(), "maybe".to_string())]);
    assert!(matches!(
        VirtualDjCrossfaderConfig::from_options(&map),
        Err(VirtualDjError::InvalidBoolean { key, value })
            if key == "crossfaderDisable" && value == "maybe"
    ));
}

#[test]
fn defaults_missing_options() {
    let config =
        VirtualDjCrossfaderConfig::from_options(Vec::<(String, String)>::new()).expect("empty map");
    assert_eq!(config, VirtualDjCrossfaderConfig::default());
    assert!(import_crossfader(&config)
        .expect("the default curve imports")
        .behavior
        .is_normal());
}

#[test]
fn options_roundtrip() {
    let config = VirtualDjCrossfaderConfig {
        crossfader_curve: CUSTOM_CURVE.to_string(),
        crossfader_custom: "0=[1,0]/1=[0,1]".to_string(),
        crossfader_hamster: true,
        crossfader_disable: false,
    };
    let map = config.to_options();
    assert_eq!(map.get(CROSSFADER_HAMSTER_KEY), Some(&"yes".to_string()));
    assert_eq!(map.get(CROSSFADER_DISABLE_KEY), Some(&"no".to_string()));
    assert_eq!(
        VirtualDjCrossfaderConfig::from_options(&map).expect("roundtrip map"),
        config
    );
}

#[test]
fn imports_straight_from_an_options_map() {
    let map = BTreeMap::from([
        ("crossfaderCurve".to_string(), "Cut".to_string()),
        ("crossfaderHamster".to_string(), "yes".to_string()),
    ]);
    let native = import_crossfader_options(&map).expect("valid map");
    assert_eq!(
        native.group_ref,
        CurveGroupRef::preset_with_param(FAST_CUT_PRESET, CUT_FADE)
    );
    assert!(native.behavior.hamster);
}

// --- export --------------------------------------------------------------

#[test]
fn exports_named_presets() {
    for name in [
        SMOOTH_CURVE,
        DEFAULT_CURVE,
        CUT_CURVE,
        SCRATCH_CURVE,
        FULL_CURVE,
    ] {
        let native = import_crossfader(&options(name, "")).expect("named preset imports");
        let exported = export_crossfader(&native).expect("named preset exports");
        assert_eq!(exported.crossfader_curve, name);
        assert!(
            exported.crossfader_custom.is_empty(),
            "`{name}` has no script"
        );
    }
}

#[test]
fn exports_an_arbitrary_cut_as_custom_points() {
    let native = import_crossfader(&options("0.3", "")).expect("amount imports");
    let exported = export_crossfader(&native).expect("custom shape exports");
    assert_eq!(exported.crossfader_curve, CUSTOM_CURVE);
    let points = parse_points(&exported.crossfader_custom).expect("valid script");
    assert_eq!(positions(&points), vec![0.0, 0.3, 0.7, 1.0]);
}

#[test]
fn exports_the_union_of_a_and_b_points() {
    let group = CurveGroup::new(vec![
        linear_curve(&[(0.0, 1.0), (0.3, 0.8), (1.0, 0.0)]),
        linear_curve(&[(0.0, 0.0), (0.7, 0.4), (1.0, 1.0)]),
    ])
    .expect("two channels");
    let native = CrossfaderConfig {
        group,
        group_ref: CurveGroupRef::custom("user_curve"),
        behavior: CrossfaderCurveBehavior::NORMAL,
    };

    let exported = export_crossfader(&native).expect("custom curve exports");
    assert_eq!(exported.crossfader_curve, CUSTOM_CURVE);
    let points = parse_points(&exported.crossfader_custom).expect("valid script");
    assert_eq!(positions(&points), vec![0.0, 0.3, 0.7, 1.0]);
    // Each channel keeps its own breakpoint; the other channel is sampled
    // there instead of being snapped to the shared positions.
    assert_close(points[1].a, 0.8);
    assert_close(points[1].b, 0.4 * 0.3 / 0.7);
    assert_close(points[2].a, 0.8 * 0.3 / 0.7);
    assert_close(points[2].b, 0.4);
}

#[test]
fn exports_hamster_and_disable_as_options() {
    let native = CrossfaderConfig {
        group: import_crossfader(&options("Smooth", ""))
            .expect("smooth imports")
            .group,
        group_ref: CurveGroupRef::preset(CONSTANT_POWER_PRESET),
        behavior: CrossfaderCurveBehavior {
            disabled: true,
            hamster: true,
        },
    };
    let exported = export_crossfader(&native).expect("exports");
    assert!(exported.crossfader_hamster);
    assert!(exported.crossfader_disable);
    assert_eq!(exported.crossfader_curve, SMOOTH_CURVE);
}

#[test]
fn rejects_a_scalar_group() {
    let native = CrossfaderConfig {
        group: CurveGroup::scalar(linear_curve(&[(0.0, 0.0), (1.0, 1.0)])).expect("scalar"),
        group_ref: CurveGroupRef::preset(LINEAR_PRESET),
        behavior: CrossfaderCurveBehavior::NORMAL,
    };
    assert!(matches!(
        export_crossfader(&native),
        Err(VirtualDjError::Curve(CurveError::ChannelCount {
            expected: 2,
            count: 1,
        }))
    ));
}

// --- roundtrips ----------------------------------------------------------

#[test]
fn roundtrips_a_named_preset() {
    let config = VirtualDjCrossfaderConfig {
        crossfader_curve: SMOOTH_CURVE.to_string(),
        crossfader_custom: String::new(),
        crossfader_hamster: true,
        crossfader_disable: false,
    };
    let native = import_crossfader(&config).expect("smooth imports");
    let exported = export_crossfader(&native).expect("smooth exports");
    assert_eq!(exported.crossfader_curve, SMOOTH_CURVE);
    assert_eq!(
        import_crossfader(&exported).expect("smooth re-imports"),
        native
    );
}

#[test]
fn roundtrips_a_custom_curve() {
    let config = options("Custom", "0=[1,0] / 0.25=[0.5,0.5] / 1=[0,1]");
    let native = import_crossfader(&config).expect("custom imports");
    let exported = export_crossfader(&native).expect("custom exports");
    assert_eq!(exported.crossfader_custom, "0=[1,0]/0.25=[0.5,0.5]/1=[0,1]");
    assert_eq!(
        import_crossfader(&exported).expect("custom re-imports"),
        native
    );
}

#[test]
fn roundtrips_a_parameterized_cut_shape() {
    let native = import_crossfader(&options("0.3", "")).expect("amount imports");
    let exported = export_crossfader(&native).expect("amount exports");
    let reimported = import_crossfader(&exported).expect("custom re-imports");
    assert_eq!(reimported.group, native.group);
    assert_eq!(reimported.behavior, native.behavior);
}

#[test]
fn roundtrips_disabled() {
    let native = import_crossfader(&options("Disabled", "")).expect("disabled imports");
    let exported = export_crossfader(&native).expect("disabled exports");
    assert_eq!(exported.crossfader_curve, SMOOTH_CURVE);
    assert!(exported.crossfader_disable);
    assert_eq!(
        import_crossfader(&exported).expect("disabled re-imports"),
        native
    );
}
