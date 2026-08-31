use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=vendor/rubberband/single/RubberBandSingle.cpp");
    println!("cargo:rerun-if-changed=build.rs");

    if try_pkg_config() {
        return;
    }

    compile_vendored();
}

fn try_pkg_config() -> bool {
    match pkg_config::Config::new()
        .atleast_version("1.8")
        .probe("rubberband")
    {
        Ok(_) => {
            println!("cargo:rustc-cfg=stretch_system_rubberband");
            true
        }
        Err(err) => {
            println!("cargo:warning=pkg-config rubberband unavailable ({err}); building vendored Rubber Band");
            false
        }
    }
}

fn compile_vendored() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let root = manifest.join("vendor/rubberband");
    let single = root.join("single/RubberBandSingle.cpp");
    if !single.exists() {
        panic!(
            "Rubber Band source missing at {}; install librubberband-dev or restore vendor/",
            single.display()
        );
    }

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++14")
        .file(&single)
        .include(&root)
        .define("ALREADY_CONFIGURED", None)
        .define("USE_BQRESAMPLER", "1")
        .define("NO_TIMING", "1")
        .define("NO_THREADING", "1")
        .define("NO_THREAD_CHECKS", "1")
        .define("RUBBERBAND_STATIC", None)
        .warnings(false);

    if env::var("CARGO_CFG_TARGET_OS").ok().as_deref() == Some("macos") {
        build.define("HAVE_VDSP", "1");
        println!("cargo:rustc-link-lib=framework=Accelerate");
    } else {
        build.define("USE_BUILTIN_FFT", "1");
        println!("cargo:rustc-link-lib=stdc++");
        println!("cargo:rustc-link-lib=m");
    }

    build.compile("rubberband_single");

    println!("cargo:rustc-cfg=stretch_vendored_rubberband");
}
