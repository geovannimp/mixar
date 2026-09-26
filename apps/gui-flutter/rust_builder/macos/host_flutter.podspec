#
# To learn more about a Podspec see http://guides.cocoapods.org/syntax/podspec.html.
# Run `pod lib lint host_flutter.podspec` to validate before publishing.
#
Pod::Spec.new do |s|
  s.name             = 'host_flutter'
  s.version          = '0.0.1'
  s.summary          = 'A new Flutter FFI plugin project.'
  s.description      = <<-DESC
A new Flutter FFI plugin project.
                       DESC
  s.homepage         = 'http://example.com'
  s.license          = { :file => '../LICENSE' }
  s.author           = { 'Your Company' => 'email@example.com' }

  # This will ensure the source files in Classes/ are included in the native
  # builds of apps using this FFI plugin. Podspec does not support relative
  # paths, so Classes contains a forwarder C file that relatively imports
  # `../src/*` so that the C sources can be shared among all target platforms.
  s.source           = { :path => '.' }
  s.source_files     = 'Classes/**/*'
  s.dependency 'FlutterMacOS'

  s.platform = :osx, '10.11'
  s.pod_target_xcconfig = { 'DEFINES_MODULE' => 'YES' }
  s.swift_version = '5.0'

  s.script_phase = {
    :name => 'Build Rust library',
    # First argument is relative path to the `rust` folder, second is name of rust library
    :script => 'sh "$PODS_TARGET_SRCROOT/../cargokit/build_pod.sh" ../../../../crates/host-flutter host_flutter',
    :execution_position => :before_compile,
    :input_files => ['${BUILT_PRODUCTS_DIR}/cargokit_phony'],
    # Let XCode know that the static library referenced in -force_load below is
    # created by this build step. Dawn is ORT WebGPU's runtime sidecar; the
    # clang_rt runtime staged next to it is a link input only, so it is not
    # declared here (build_pod.sh picks whichever name it finds).
    :output_files => [
      "${BUILT_PRODUCTS_DIR}/libhost_flutter.a",
      "${BUILT_PRODUCTS_DIR}/libhost_flutter_rust.a",
      "${BUILT_PRODUCTS_DIR}/libwebgpu_dawn.dylib",
    ],
  }
  # force_load of the static Rust lib does not pull cargo's framework link args;
  # midir/cpal need these for the Flutter macOS link.
  s.frameworks = 'AudioToolbox', 'CoreAudio', 'CoreMIDI'
  s.pod_target_xcconfig = {
    'DEFINES_MODULE' => 'YES',
    # Flutter.framework does not contain a i386 slice.
    'EXCLUDED_ARCHS[sdk=iphonesimulator*]' => 'i386',
    # Apple Silicon only: ort-sys (2.0.0-rc.12) has no x86_64-apple-darwin
    # prebuilt, so cargokit would fail building the x86_64 slice of the Rust lib.
    # build_pod.sh builds one Rust target per entry of $ARCHS, so set ARCHS
    # directly rather than relying on EXCLUDED_ARCHS to filter $ARCHS. Keep in
    # sync with macos/Runner/Configs/AppInfo.xcconfig.
    'ARCHS' => 'arm64',
    # ONNX Runtime's macOS prebuilt is built for macOS 13.4, so linking it into
    # a lower deployment target warns on every ORT object. This is a build
    # setting rather than `s.platform` because the generated Podfile pins
    # `platform :osx, '12.0'` and CocoaPods rejects a podspec platform above it.
    'MACOSX_DEPLOYMENT_TARGET' => '13.4',
    # -force_load only the Rust half of the static lib; ONNX Runtime's members
    # come in lazily from libhost_flutter.a.
    #
    # cargokit merges the crate's objects with every native static library cargo
    # linked, and the ORT prebuilt ships members twice -- two builds of
    # onnx-ml.pb.cc.o alone define ~776 of the same strong symbols -- so
    # -force_load of the merged archive dies with "756 duplicate symbols". Those
    # repeats are latent for a normal link, which is how cargo links this exact
    # archive on Linux and Windows (and how the Rust cdylib link on this machine
    # already succeeds), because only the members still needed are pulled in.
    # build_pod.sh splits the Rust objects (rustc's *.rcgu.o) into
    # libhost_flutter_rust.a, since nothing references them and they still have
    # to be forced in.
    #
    # ort-sys also links `-lc++`, `-lclang_rt.osx` and Foundation for ORT's
    # static macOS build. cargo applied them to the Rust link, but this Xcode
    # link is the one that has to resolve ~250 libc++/libc++abi symbols plus
    # ORT's compiler-rt builtins. build_pod.sh stages the clang_rt runtime into
    # ${BUILT_PRODUCTS_DIR} (it lives inside the toolchain or SDK and its path
    # moves with the Xcode version, so it is discovered there, not hardcoded).
    'OTHER_LDFLAGS' => '-force_load ${BUILT_PRODUCTS_DIR}/libhost_flutter_rust.a -L${BUILT_PRODUCTS_DIR} -lhost_flutter -lwebgpu_dawn -lclang_rt.osx -lc++ -framework Foundation',
    'LD_RUNPATH_SEARCH_PATHS' => '$(inherited) @loader_path @executable_path/../Frameworks',
  }
end
