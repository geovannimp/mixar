#!/usr/bin/env bash
set -euo pipefail

# Last official Intel macOS runtime. Keep the API feature in analyzer-stems's
# x86_64 macOS dependency in sync with this version.
version=1.23.2
checksum=d10359e16347b57d9959f7e80a225a5b4a66ed7d7e007274a15cae86836485a6
if [ "$#" -ne 1 ] || [ -z "${1:-}" ]; then
  echo "error: usage: prepare_macos_ort.sh <destination-directory>" >&2
  exit 1
fi
destination="$1"
runtime=libonnxruntime.dylib
library="$destination/$runtime"

if [ ! -f "$destination/.version-$version" ] || [ ! -f "$library" ]; then
  temporary=$(mktemp -d)
  trap 'rm -rf "$temporary"' EXIT
  archive="onnxruntime-osx-x86_64-$version"
  # Microsoft's release asset for this last Intel macOS build is no longer
  # downloadable (the release 404s), so fall back to a bit-for-bit mirror. The
  # pinned SHA-256 below is what makes a mirror acceptable: content that does
  # not match Microsoft's published digest never gets installed.
  urls=(
    "https://github.com/microsoft/onnxruntime/releases/download/v$version/$archive.tgz"
    "https://dl.photoprism.app/onnx/runtime/v$version/$archive.tgz"
  )
  downloaded=""
  for url in "${urls[@]}"; do
    if curl --fail --silent --show-error --location --retry 3 "$url" \
      --output "$temporary/runtime.tgz"; then
      downloaded="$url"
      break
    fi
    echo "warning: ONNX Runtime $version is not available at $url" >&2
  done
  if [ -z "$downloaded" ]; then
    echo "error: could not download ONNX Runtime $version from any source:" >&2
    printf '  %s\n' "${urls[@]}" >&2
    exit 1
  fi
  echo "ONNX Runtime $version downloaded from $downloaded" >&2
  echo "$checksum  $temporary/runtime.tgz" | shasum -a 256 --check
  tar -xzf "$temporary/runtime.tgz" -C "$temporary"
  mkdir -p "$destination"
  # Dereference the archive's versioned symlink and give the bundled copy a
  # relocatable, unversioned install name before Rust records its dependency.
  cp -L "$temporary/$archive/lib/$runtime" "$temporary/$runtime"
  install_name_tool -id "@rpath/$runtime" "$temporary/$runtime"
  lipo "$temporary/$runtime" -verify_arch x86_64
  mv "$temporary/$runtime" "$library"
  touch "$destination/.version-$version"
fi

lipo "$library" -verify_arch x86_64
