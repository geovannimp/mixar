//! Runtime search path for ORT WebGPU Dawn sidecar next to this cdylib.

fn main() {
    // `analyzer-stems` default `webgpu` links `libwebgpu_dawn.so` / `.dylib`.
    // Flutter bundles that sidecar beside `libhost_flutter.so`; without $ORIGIN /
    // @loader_path, dlopen of the host fails before main runs.
    #[cfg(target_os = "linux")]
    println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");

    #[cfg(target_os = "macos")]
    println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path");
}
