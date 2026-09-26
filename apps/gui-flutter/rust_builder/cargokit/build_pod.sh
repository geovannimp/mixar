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

# Split the Rust objects out of the merged static lib. cargokit merges the
# crate's objects with every native static library cargo linked, and the ONNX
# Runtime prebuilt ships members twice -- two builds of onnx-ml.pb.cc.o alone
# define ~776 of the same strong symbols -- so the podspec can only force_load
# the Rust half. The native members are pulled in lazily from the merged
# archive, which is how cargo already links this exact archive on Linux and
# Windows, so the repeats stay latent.
#
# rustc names its own objects <...>.rcgu.o and nothing in ONNX Runtime does, so
# the split is a name filter rather than a heuristic about provenance.
MERGED_LIB="$CARGOKIT_OUTPUT_DIR/libhost_flutter.a"
RUST_LIB="$CARGOKIT_OUTPUT_DIR/libhost_flutter_rust.a"
if [ -f "$MERGED_LIB" ]; then
  # cargokit always runs `lipo -create` over the static lib, so what lands in
  # ${BUILT_PRODUCTS_DIR} is a fat archive even for a single arch -- and ar
  # refuses those ("is a fat file"). Thin it per arch first, split each slice,
  # and lipo the Rust halves back together when there is more than one.
  SPLIT_DIR="$(mktemp -d)"
  RUST_SLICE_COUNT=0
  LAST_SLICE=""
  for arch in ${ARCHS:-arm64}; do
    SLICE="$SPLIT_DIR/lib-$arch.a"
    if ! lipo -thin "$arch" "$MERGED_LIB" -output "$SLICE" 2>/dev/null; then
      echo "warning: no $arch slice in $MERGED_LIB" >&2
      continue
    fi
    MEMBERS="$(ar t "$SLICE" | grep -c '\.rcgu\.o$' || true)"
    if [ "${MEMBERS:-0}" -eq 0 ]; then
      echo "warning: no .rcgu.o members in the $arch slice of $MERGED_LIB" >&2
      continue
    fi
    OBJ_DIR="$SPLIT_DIR/objs-$arch"
    mkdir -p "$OBJ_DIR"
    # ar takes a few thousand names at most, so extract in batches rather than
    # handing it the whole member list. Object names carry no spaces.
    ar t "$SLICE" | grep '\.rcgu\.o$' | xargs -n 200 sh -c 'cd "$1" || exit 1; a="$2"; shift 2; ar x "$a" "$@"' _ "$OBJ_DIR" "$SLICE"
    RUST_SLICE="$SPLIT_DIR/rust-$arch.a"
    rm -f "$RUST_SLICE"
    find "$OBJ_DIR" -name '*.rcgu.o' | xargs -n 300 ar -rcs "$RUST_SLICE"
    rm -rf "$OBJ_DIR"
    LAST_SLICE="$RUST_SLICE"
    RUST_SLICE_COUNT=$((RUST_SLICE_COUNT + 1))
    echo "info: split $MEMBERS Rust objects out of the $arch slice"
  done
  if [ "$RUST_SLICE_COUNT" -gt 0 ]; then
    rm -f "$RUST_LIB"
    if [ "$RUST_SLICE_COUNT" -eq 1 ]; then
      cp "$LAST_SLICE" "$RUST_LIB"
    else
      lipo -create "$SPLIT_DIR"/rust-*.a -output "$RUST_LIB"
    fi
  else
    echo "warning: the link will fail with 'no such file: $RUST_LIB'" >&2
  fi
  rm -rf "$SPLIT_DIR"
fi

# Stage ONNX Runtime's C++ runtime where the pod's `-lclang_rt.osx` can see it.
# ORT's static objects reference compiler-rt builtins (e.g.
# __isPlatformVersionAtLeast, which clang emits for @available in the CoreML EP's
# Objective-C++) that neither the OS libc++ nor libSystem export -- that is why
# ort-sys links `-lclang_rt.osx` on apple-darwin. The pod's Xcode link only reads
# OTHER_LDFLAGS, so the podspec repeats the flag, and the runtime itself is
# staged here: it lives inside the toolchain or SDK, and both the layout and the
# name move with the Xcode version (26.6 has no libclang_rt.osx.dylib at all), so
# it is discovered rather than assumed. The static archive is preferred: it needs
# no shipping, signing or rpath.
CLANG_RT_CLANG="$(xcrun -f clang 2>/dev/null)" || true
CLANG_RT_ROOT1="$(xcrun clang --print-resource-dir 2>/dev/null)/lib/darwin" || true
CLANG_RT_ROOT2="$(dirname "$CLANG_RT_CLANG")/../lib"
CLANG_RT_ROOT3="$(xcrun --show-sdk-path 2>/dev/null)/usr/lib" || true
CLANG_RT_SRC=""
for pattern in libclang_rt.osx.a libclang_rt.macosx.a libclang_rt.osx.dylib libclang_rt.macosx.dylib; do
  # Each root is quoted: Xcode can sit under a path containing spaces, which word
  # splitting would tear apart.
  for dir in "$CLANG_RT_ROOT1" "$CLANG_RT_ROOT2" "$CLANG_RT_ROOT3"; do
    if [ -z "$CLANG_RT_SRC" ] && [ -d "$dir" ]; then
      CLANG_RT_SRC="$(find "$dir" -maxdepth 5 -name "$pattern" 2>/dev/null | head -1)"
    fi
  done
  if [ -n "$CLANG_RT_SRC" ]; then
    break
  fi
done
if [ -n "$CLANG_RT_SRC" ]; then
  # Normalise the name so the podspec's `-lclang_rt.osx` resolves even when the
  # toolchain calls the runtime macosx.
  case "$CLANG_RT_SRC" in
    *.a) CLANG_RT_STAGED="$CARGOKIT_OUTPUT_DIR/libclang_rt.osx.a" ;;
    *)   CLANG_RT_STAGED="$CARGOKIT_OUTPUT_DIR/libclang_rt.osx.dylib" ;;
  esac
  # Drop any other variant an earlier build staged: ld looks for the dylib before
  # the archive in a directory, and the sidecar loop below ships whatever is here.
  rm -f "$CARGOKIT_OUTPUT_DIR/libclang_rt.osx.a" "$CARGOKIT_OUTPUT_DIR/libclang_rt.osx.dylib"
  cp -f "$CLANG_RT_SRC" "$CLANG_RT_STAGED"
  echo "info: staged ORT C++ runtime from $CLANG_RT_SRC"
  # Dynamic variant only: it has to be reachable through the runner rpaths, which
  # look in the app's Frameworks.
  case "$CLANG_RT_STAGED" in
    *.dylib) install_name_tool -id @rpath/libclang_rt.osx.dylib "$CLANG_RT_STAGED" 2>/dev/null || true ;;
  esac
else
  echo "warning: no libclang_rt.osx runtime found under:" >&2
  echo "warning:   $CLANG_RT_ROOT1" >&2
  echo "warning:   $CLANG_RT_ROOT2" >&2
  echo "warning:   $CLANG_RT_ROOT3" >&2
  echo "warning: the host_flutter link will fail with 'library not found for -lclang_rt.osx'" >&2
fi

# Embed the ORT runtime sidecars beside the app Frameworks when Xcode provides
# the paths, and sign them: on Apple Silicon every Mach-O in the bundle needs a
# signature to be mapped, and nothing else signs dylibs copied by a build phase.
if [ -n "${TARGET_BUILD_DIR:-}" ] && [ -n "${FRAMEWORKS_FOLDER_PATH:-}" ]; then
  for sidecar in libwebgpu_dawn.dylib libclang_rt.osx.dylib; do
    if [ -f "$CARGOKIT_OUTPUT_DIR/$sidecar" ]; then
      mkdir -p "${TARGET_BUILD_DIR}/${FRAMEWORKS_FOLDER_PATH}"
      cp -f "$CARGOKIT_OUTPUT_DIR/$sidecar" "${TARGET_BUILD_DIR}/${FRAMEWORKS_FOLDER_PATH}/"
      codesign --force --sign - "${TARGET_BUILD_DIR}/${FRAMEWORKS_FOLDER_PATH}/${sidecar}" 2>/dev/null || true
    fi
  done
fi

# Make a symlink from built framework to phony file, which will be used as input to
# build script. This should force rebuild (podspec currently doesn't support alwaysOutOfDate
# attribute on custom build phase)
ln -fs "$OBJROOT/XCBuildData/build.db" "${BUILT_PRODUCTS_DIR}/cargokit_phony"
ln -fs "${BUILT_PRODUCTS_DIR}/${EXECUTABLE_PATH}" "${BUILT_PRODUCTS_DIR}/cargokit_phony_out"
