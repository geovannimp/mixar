//! Rubber Band C API FFI (subset used for realtime key lock).

#![allow(non_camel_case_types)]

use std::os::raw::{c_double, c_int, c_uint};

pub type RubberBandState = *mut std::os::raw::c_void;
pub type RubberBandOptions = c_int;

pub const RUBBERBAND_OPTION_PROCESS_REAL_TIME: RubberBandOptions = 0x0000_0001;
pub const RUBBERBAND_OPTION_THREADING_NEVER: RubberBandOptions = 0x0001_0000;
pub const RUBBERBAND_OPTION_CHANNELS_TOGETHER: RubberBandOptions = 0x1000_0000;
pub const RUBBERBAND_OPTION_ENGINE_FINER: RubberBandOptions = 0x2000_0000;
pub const RUBBERBAND_OPTION_PITCH_HIGH_CONSISTENCY: RubberBandOptions = 0x0400_0000;

unsafe extern "C" {
    pub fn rubberband_new(
        sample_rate: c_uint,
        channels: c_uint,
        options: RubberBandOptions,
        initial_time_ratio: c_double,
        initial_pitch_scale: c_double,
    ) -> RubberBandState;

    pub fn rubberband_delete(state: RubberBandState);
    pub fn rubberband_reset(state: RubberBandState);

    pub fn rubberband_set_time_ratio(state: RubberBandState, ratio: c_double);
    pub fn rubberband_set_pitch_scale(state: RubberBandState, scale: c_double);

    pub fn rubberband_get_preferred_start_pad(state: RubberBandState) -> c_uint;
    pub fn rubberband_get_start_delay(state: RubberBandState) -> c_uint;
    pub fn rubberband_get_samples_required(state: RubberBandState) -> c_uint;
    pub fn rubberband_set_max_process_size(state: RubberBandState, samples: c_uint);

    pub fn rubberband_process(
        state: RubberBandState,
        input: *const *const f32,
        samples: c_uint,
        final_: c_int,
    );

    pub fn rubberband_available(state: RubberBandState) -> c_int;
    pub fn rubberband_retrieve(
        state: RubberBandState,
        output: *const *mut f32,
        samples: c_uint,
    ) -> c_uint;
}
