//! On-device EP availability probe for Android.
//!
//! ```text
//! cargo build -p analyzer-stems --target aarch64-linux-android --example android_ep_probe
//! adb push target/aarch64-linux-android/debug/examples/android_ep_probe /data/local/tmp/
//! adb push libonnxruntime.so /data/local/tmp/
//! adb shell ORT_DYLIB_PATH=/data/local/tmp/libonnxruntime.so /data/local/tmp/android_ep_probe
//! ```

use analyzer_stems::{preferred_ep_labels, preferred_live_ep_labels, FORCE_EP_ENV};
#[cfg(target_os = "android")]
use analyzer_stems::{ANDROID_ORT_DYLIB, ORT_DYLIB_PATH_ENV};

fn main() {
    #[cfg(target_os = "android")]
    let builder = {
        let path =
            std::env::var(ORT_DYLIB_PATH_ENV).unwrap_or_else(|_| ANDROID_ORT_DYLIB.to_owned());
        ort::init_from(&path).unwrap_or_else(|err| {
            panic!(
                "load ONNX Runtime '{path}' (set {ORT_DYLIB_PATH_ENV} to an absolute path): {err}"
            )
        })
    };
    #[cfg(not(target_os = "android"))]
    let builder = ort::init();
    let _env = builder
        .build()
        .unwrap_or_else(|err| panic!("environment: {err}"));

    println!("cascade={:?}", preferred_ep_labels());
    println!("live_cascade={:?}", preferred_live_ep_labels());
    report_availability();
    println!("force={:?}", std::env::var(FORCE_EP_ENV).ok());
}

/// Print `is_available()` for every EP this build can probe.
///
/// Android enables all seven (the AAR carries NNAPI/XNNPACK; the custom-build
/// EPs report `false` until a runtime ships them). Desktop builds probe CPU
/// and, with the `webgpu` feature, WebGPU — the other EP modules do not exist
/// without their cargo feature.
fn report_availability() {
    use ort::ep::ExecutionProvider as _;

    macro_rules! report {
        ($($ty:ident => $label:expr),* $(,)?) => {
            $(println!("{}_available={}", $label, ort::ep::$ty::is_available().unwrap_or(false));)*
        };
    }

    #[cfg(target_os = "android")]
    report!(
        NNAPI => analyzer_stems::NNAPI_EP_LABEL,
        XNNPACK => analyzer_stems::XNNPACK_EP_LABEL,
        ACL => analyzer_stems::ACL_EP_LABEL,
        QNN => analyzer_stems::QNN_EP_LABEL,
        TVM => analyzer_stems::TVM_EP_LABEL,
        Azure => analyzer_stems::AZURE_EP_LABEL,
        CPU => analyzer_stems::CPU_EP_LABEL,
    );
    #[cfg(not(target_os = "android"))]
    {
        report!(CPU => analyzer_stems::CPU_EP_LABEL);
        #[cfg(feature = "webgpu")]
        report!(WebGPU => analyzer_stems::WEBGPU_EP_LABEL);
    }
}
