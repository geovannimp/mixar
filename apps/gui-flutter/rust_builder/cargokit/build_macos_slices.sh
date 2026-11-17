#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -lt 2 ] || [ -z "${1:-}" ] || [ -z "${2:-}" ]; then
  echo "error: usage: build_macos_slices.sh <manifest-directory> <library-name>" >&2
  exit 1
fi
library_name="$2"
intel_runtime=libonnxruntime.dylib
arm_runtime=libwebgpu_dawn.dylib
sidecar_libraries=("$intel_runtime" "$arm_runtime")

basedir=$(cd "$(dirname "$0")"; pwd -P)
output="$CARGOKIT_OUTPUT_DIR"
slices="$CARGOKIT_TARGET_TEMP_DIR/macos-slices"
read -r -a architectures <<< "${CARGOKIT_DARWIN_ARCHS:-}" || true
if [ "${#architectures[@]}" -eq 0 ]; then
  echo "error: no macOS architectures requested" >&2
  exit 1
fi

mkdir -p "$output"
# Do not retain a runtime from a different architecture/configuration.
for sidecar in "${sidecar_libraries[@]}"; do
  rm -f "$output/$sidecar"
done
archives=()
for architecture in "${architectures[@]}"; do
  case "$architecture" in
    arm64) required_sidecar="$arm_runtime" ;;
    x86_64) required_sidecar="$intel_runtime" ;;
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
      cp "$ORT_LIB_LOCATION/$intel_runtime" "$slice/"
    fi
  )
  archive="$slice/lib${library_name}.a"
  lipo -verify_arch "$architecture" "$archive"
  archives+=("$archive")
  # Cargo's cached artifacts can contain the other architecture's runtime.
  # Ignore those optional leftovers, but never skip a missing or wrong-arch
  # runtime that this slice actually links.
  for sidecar in "${sidecar_libraries[@]}"; do
    if [ "$sidecar" != "$required_sidecar" ]; then
      continue
    fi
    lipo -verify_arch "$architecture" "$slice/$sidecar"
    cp "$slice/$sidecar" "$output/"
  done
done

lipo -create "${archives[@]}" -output "$output/lib${library_name}.a"
lipo -verify_arch "${architectures[@]}" "$output/lib${library_name}.a"
