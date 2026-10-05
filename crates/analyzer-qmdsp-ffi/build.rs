//! Compile Mixxx's vendored qm-dsp + the C ABI shim.
//!
//! qm-dsp source is not vendored here. Point `QMDSP_SOURCE_DIR` at a checkout of
//! Mixxx's `lib/qm-dsp` (default `~/.cache/mixar/mixxx-src/lib/qm-dsp`). When the
//! source is missing, the crate builds empty and [`crate::QmdspFfiAnalyzer::from_env`]
//! reports it unavailable, so the workspace still builds without qm-dsp.

use std::path::{Path, PathBuf};

/// C++ sources needed for beats + key (segmentation/mfcc/rhythm/tonal/wavelet and
/// the hmm/clapack code they pull in are excluded).
const CPP_SOURCES: &[&str] = &[
    "base/KaiserWindow.cpp",
    "base/Pitch.cpp",
    "base/SincWindow.cpp",
    "dsp/chromagram/Chromagram.cpp",
    "dsp/chromagram/ConstantQ.cpp",
    "dsp/keydetection/GetKeyMode.cpp",
    "dsp/onsets/DetectionFunction.cpp",
    "dsp/onsets/PeakPicking.cpp",
    "dsp/phasevocoder/PhaseVocoder.cpp",
    "dsp/rateconversion/Decimator.cpp",
    "dsp/rateconversion/DecimatorB.cpp",
    "dsp/rateconversion/Resampler.cpp",
    "dsp/signalconditioning/DFProcess.cpp",
    "dsp/signalconditioning/Filter.cpp",
    "dsp/signalconditioning/FiltFilt.cpp",
    "dsp/signalconditioning/Framer.cpp",
    "dsp/tempotracking/DownBeat.cpp",
    "dsp/tempotracking/TempoTrack.cpp",
    "dsp/tempotracking/TempoTrackV2.cpp",
    "dsp/transforms/DCT.cpp",
    "dsp/transforms/FFT.cpp",
    "maths/Correlation.cpp",
    "maths/CosineDistance.cpp",
    "maths/KLDivergence.cpp",
    "maths/MathUtilities.cpp",
];

/// kissfft (C).
const C_SOURCES: &[&str] = &["ext/kissfft/kiss_fft.c", "ext/kissfft/tools/kiss_fftr.c"];

fn source_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("QMDSP_SOURCE_DIR") {
        if !dir.trim().is_empty() {
            return PathBuf::from(dir);
        }
    }
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join(".cache/mixar/mixxx-src/lib/qm-dsp")
}

fn common(build: &mut cc::Build, src: &Path) {
    build
        .include(src)
        .include(src.join("ext/kissfft"))
        .include(src.join("ext/kissfft/tools"))
        .include("src")
        .define("kiss_fft_scalar", "double")
        .warnings(false);
    if matches!(
        std::env::var("CARGO_CFG_TARGET_ARCH").as_deref(),
        Ok("x86_64") | Ok("x86")
    ) {
        build.flag("-msse").flag("-msse2").flag("-mfpmath=sse");
    }
}

fn main() {
    println!("cargo:rerun-if-env-changed=QMDSP_SOURCE_DIR");
    println!("cargo:rerun-if-changed=src/qm_shim.cpp");

    let src = source_dir();
    if !src.join("dsp/keydetection/GetKeyMode.cpp").exists() {
        println!(
            "cargo:warning=qm-dsp source not found at {} (set QMDSP_SOURCE_DIR); \
             analyzer-qmdsp-ffi disabled",
            src.display()
        );
        return;
    }

    let mut cpp = cc::Build::new();
    cpp.cpp(true).std("c++17");
    common(&mut cpp, &src);
    for source in CPP_SOURCES {
        cpp.file(src.join(source));
    }
    cpp.file("src/qm_shim.cpp");
    cpp.compile("qm_shim_cpp");

    let mut c = cc::Build::new();
    c.cpp(false);
    common(&mut c, &src);
    for source in C_SOURCES {
        c.file(src.join(source));
    }
    c.compile("qm_shim_c");

    match std::env::var("CARGO_CFG_TARGET_OS").as_deref() {
        Ok("linux") => println!("cargo:rustc-link-lib=dylib=stdc++"),
        Ok("macos") => println!("cargo:rustc-link-lib=dylib=c++"),
        _ => {}
    }

    println!("cargo:rustc-cfg=qm_dsp_available");
    println!("cargo:rustc-check-cfg=cfg(qm_dsp_available)");
}
