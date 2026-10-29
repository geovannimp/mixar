//! Wire constants that mirror DSP tables across the crate boundary.
//!
//! `bus-api` is the host-facing schema and must not depend on `engine-dsp`,
//! so the counts are declared twice; `engine-core` links both and pins them
//! together here, where a table change is caught at test time instead of
//! silently shifting the normalized value a control publishes.

#[test]
fn fx_constants_match_the_dsp_tables() {
    assert_eq!(bus_api::FX_SLOT_COUNT, engine_dsp::FX_SLOT_COUNT);
    assert_eq!(bus_api::FX_PARAM_COUNT, engine_dsp::FX_PARAM_COUNT);
    assert_eq!(
        bus_api::FX_DIVISION_COUNT,
        engine_dsp::fx::echo::ECHO_DIVISIONS.len(),
        "FX_DIVISION_COUNT must mirror the Echo beat-division table",
    );
}

#[test]
fn boot_fx_slot_matches_the_dsp_chain() {
    let chain = engine_dsp::FxChain::new(48_000);
    let slot = chain.slot(0).expect("slot 0");
    let boot = bus_api::FxSlotState::boot();

    assert_eq!(slot.kind(), engine_dsp::FxKind::Echo);
    assert_eq!(boot.kind, bus_api::FxKind::Echo);
    assert!(!boot.enabled, "the unit boots loaded but unarmed");
    assert!((boot.wet - slot.wet()).abs() < f32::EPSILON);
    assert_eq!(boot.params, slot.params());
}
