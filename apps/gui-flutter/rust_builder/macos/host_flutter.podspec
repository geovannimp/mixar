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
    # created by this build step. Dawn is ORT WebGPU's runtime sidecar.
    :output_files => [
      "${BUILT_PRODUCTS_DIR}/libhost_flutter.a",
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
    'OTHER_LDFLAGS' => '-force_load ${BUILT_PRODUCTS_DIR}/libhost_flutter.a -L${BUILT_PRODUCTS_DIR} -lwebgpu_dawn',
    'LD_RUNPATH_SEARCH_PATHS' => '$(inherited) @loader_path @executable_path/../Frameworks',
  }
end
