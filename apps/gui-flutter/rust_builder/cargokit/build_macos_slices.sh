#!/usr/bin/env bash
set -euo pipefail

basedir=$(cd "$(dirname "$0")"; pwd -P)
output="$CARGOKIT_OUTPUT_DIR"
slices="$CARGOKIT_TARGET_TEMP_DIR/macos-slices"
read -r -a architectures <<< "$CARGOKIT_DARWIN_ARCHS"
if [ "${#architectures[@]}" -eq 0 ]; then
  echo "error: no macOS architectures requested" >&2
  exit 1
fi

mkdir -p "$output"
# Do not retain a runtime from a different architecture/configuration.
rm -f "$output/libonnxruntime.dylib" "$output/libwebgpu_dawn.dylib"
archives=()
for architecture in "${architectures[@]}"; do
  case "$architecture" in
    arm64|x86_64) ;;
    *) echo "error: unsupported macOS architecture: $architecture" >&2; exit 1 ;;
  esac
  slice="$slices/$architecture"
  rm -rf "$slice"
  mkdir -p "$slice"
  (
    export CARGOKIT_DARWIN_ARCHS="$architecture"
    export CARGOKIT_OUTPUT_DIR="$slice"
    if [ "$architecture" = x86_64 ]; then
      export ORT_LIB_LOCATION="$CARGOKIT_TARGET_TEMP_DIR/onnxruntime-x86_64"
      # ORT_LIB_PATH takes precedence over ORT_LIB_LOCATION in ort-sys.
      unset ORT_LIB_PATH
      export ORT_PREFER_DYNAMIC_LINK=1
      bash "$basedir/prepare_macos_ort.sh" "$ORT_LIB_LOCATION"
    fi
    sh "$basedir/run_build_tool.sh" build-pod "$@"
    if [ "$architecture" = x86_64 ]; then
      cp "$ORT_LIB_LOCATION/libonnxruntime.dylib" "$slice/"
    fi
  )
  archive="$slice/lib${2}.a"
  lipo -verify_arch "$architecture" "$archive"
  archives+=("$archive")
  # These runtimes intentionally have different architectures: only the Intel
  # executable links onnxruntime; only Apple Silicon links WebGPU Dawn.
  for sidecar in libonnxruntime.dylib libwebgpu_dawn.dylib; do
    if [ -f "$slice/$sidecar" ]; then
      lipo -verify_arch "$architecture" "$slice/$sidecar"
      cp "$slice/$sidecar" "$output/"
    fi
  done
done

lipo -create "${archives[@]}" -output "$output/lib${2}.a"
lipo -verify_arch "${architectures[@]}" "$output/lib${2}.a"
