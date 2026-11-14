#!/usr/bin/env bash
set -euo pipefail

# Last official Intel macOS runtime. Keep the API feature in analyzer-stems's
# x86_64 macOS dependency in sync with this version.
version=1.23.2
checksum=d10359e16347b57d9959f7e80a225a5b4a66ed7d7e007274a15cae86836485a6
destination="$1"
library="$destination/libonnxruntime.dylib"

if [ ! -f "$destination/.version-$version" ] || [ ! -f "$library" ]; then
  temporary=$(mktemp -d)
  trap 'rm -rf "$temporary"' EXIT
  archive="onnxruntime-osx-x86_64-$version"
  curl --fail --location --retry 3 \
    "https://github.com/microsoft/onnxruntime/releases/download/v$version/$archive.tgz" \
    --output "$temporary/runtime.tgz"
  echo "$checksum  $temporary/runtime.tgz" | shasum -a 256 --check
  tar -xzf "$temporary/runtime.tgz" -C "$temporary"
  mkdir -p "$destination"
  # Dereference the archive's versioned symlink and give the bundled copy a
  # relocatable, unversioned install name before Rust records its dependency.
  cp -L "$temporary/$archive/lib/libonnxruntime.dylib" "$temporary/libonnxruntime.dylib"
  install_name_tool -id @rpath/libonnxruntime.dylib "$temporary/libonnxruntime.dylib"
  lipo -verify_arch x86_64 "$temporary/libonnxruntime.dylib"
  mv "$temporary/libonnxruntime.dylib" "$library"
  touch "$destination/.version-$version"
fi

lipo -verify_arch x86_64 "$library"
