#!/bin/sh
set -e

BASEDIR=$(dirname "$0")

# Workaround for https://github.com/dart-lang/pub/issues/4010
BASEDIR=$(cd "$BASEDIR" ; pwd -P)

# Remove XCode SDK from path. Otherwise this breaks tool compilation when building iOS project
NEW_PATH=`echo $PATH | tr ":" "\n" | grep -v "Contents/Developer/" | tr "\n" ":"`

export PATH=${NEW_PATH%?} # remove trailing :

env

# Platform name (macosx, iphoneos, iphonesimulator)
export CARGOKIT_DARWIN_PLATFORM_NAME=$PLATFORM_NAME

# Arctive architectures (arm64, armv7, x86_64), space separated.
export CARGOKIT_DARWIN_ARCHS=$ARCHS

# Current build configuration (Debug, Release)
export CARGOKIT_CONFIGURATION=$CONFIGURATION

# Path to directory containing Cargo.toml.
export CARGOKIT_MANIFEST_DIR=$PODS_TARGET_SRCROOT/$1

# Temporary directory for build artifacts.
export CARGOKIT_TARGET_TEMP_DIR=$TARGET_TEMP_DIR

# Output directory for final artifacts.
export CARGOKIT_OUTPUT_DIR=$PODS_CONFIGURATION_BUILD_DIR/$PRODUCT_NAME

# Directory to store built tool artifacts.
export CARGOKIT_TOOL_TEMP_DIR=$TARGET_TEMP_DIR/build_tool

# Directory inside root project. Not necessarily the top level directory of root project.
export CARGOKIT_ROOT_PROJECT_DIR=$SRCROOT

FLUTTER_EXPORT_BUILD_ENVIRONMENT=(
  "$PODS_ROOT/../Flutter/ephemeral/flutter_export_environment.sh" # macOS
  "$PODS_ROOT/../Flutter/flutter_export_environment.sh" # iOS
)

for path in "${FLUTTER_EXPORT_BUILD_ENVIRONMENT[@]}"
do
  if [[ -f "$path" ]]; then
    source "$path"
  fi
done

sh "$BASEDIR/run_build_tool.sh" build-pod "$@"

# Stage ONNX Runtime's C++ runtime dylib next to libhost_flutter.a. ort-sys
# emits `-lc++` and `-lclang_rt.osx` when it links ORT's static build on macOS
# (libc++'s hidden inline instantiations only exist in the toolchain's
# libclang_rt, not in the OS libc++), but this pod force-loads the .a into an
# Xcode link that reads only OTHER_LDFLAGS, so the podspec repeats those flags.
# The dylib lives under the Xcode version directory, so the path can only be
# resolved here, at build time.
CLANG_RT_SRC="$(xcrun --sdk macosx clang --print-resource-dir 2>/dev/null)/lib/darwin/libclang_rt.osx.dylib"
if [ -f "$CLANG_RT_SRC" ]; then
  cp -f "$CLANG_RT_SRC" "$CARGOKIT_OUTPUT_DIR/libclang_rt.osx.dylib"
  # Ship it from the app's Frameworks; the runner rpaths only look there.
  install_name_tool -id @rpath/libclang_rt.osx.dylib \
    "$CARGOKIT_OUTPUT_DIR/libclang_rt.osx.dylib" 2>/dev/null || true
else
  echo "warning: libclang_rt.osx.dylib not found at '$CLANG_RT_SRC'" >&2
  echo "warning: the host_flutter link will fail with 'library not found for -lclang_rt.osx'" >&2
fi

# Embed the ORT runtime sidecars beside the app Frameworks when Xcode provides
# the paths, and sign them: on Apple Silicon every Mach-O in the bundle needs a
# signature to be mapped, and nothing else signs dylibs copied by a build phase.
if [ -n "${TARGET_BUILD_DIR:-}" ] && [ -n "${FRAMEWORKS_FOLDER_PATH:-}" ]; then
  for sidecar in libwebgpu_dawn.dylib libclang_rt.osx.dylib; do
    [ -f "$CARGOKIT_OUTPUT_DIR/$sidecar" ] || continue
    mkdir -p "${TARGET_BUILD_DIR}/${FRAMEWORKS_FOLDER_PATH}"
    cp -f "$CARGOKIT_OUTPUT_DIR/$sidecar" "${TARGET_BUILD_DIR}/${FRAMEWORKS_FOLDER_PATH}/"
    codesign --force --sign - "${TARGET_BUILD_DIR}/${FRAMEWORKS_FOLDER_PATH}/${sidecar}" 2>/dev/null || true
  done
fi

# Make a symlink from built framework to phony file, which will be used as input to
# build script. This should force rebuild (podspec currently doesn't support alwaysOutOfDate
# attribute on custom build phase)
ln -fs "$OBJROOT/XCBuildData/build.db" "${BUILT_PRODUCTS_DIR}/cargokit_phony"
ln -fs "${BUILT_PRODUCTS_DIR}/${EXECUTABLE_PATH}" "${BUILT_PRODUCTS_DIR}/cargokit_phony_out"
