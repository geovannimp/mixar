#[cfg(target_os = "android")]
mod android;
pub mod api;
mod app_paths;
mod bridge;
#[allow(clippy::wildcard_imports, clippy::unwrap_used)] // generated code
mod frb_generated;
// The JNI context handoff belongs to Android, but its state machine is compiled
// for tests too, so the retry and reporting rules are covered on every host.
#[cfg(any(test, target_os = "android"))]
mod jni_context;

#[cfg(any(test, feature = "test-utils"))]
#[doc(hidden)]
pub mod testing;

// Android loads Microsoft's ONNX Runtime AAR, which has no WebGPU EP. Enabling
// the desktop GPU provider there resolves symbols the library does not export,
// which surfaces as a `dlopen`/registration failure instead of a build error.
#[cfg(all(target_os = "android", feature = "stems-webgpu"))]
compile_error!("`stems-webgpu` must not be enabled on Android; build with `stems` only");
