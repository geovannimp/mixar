#!/usr/bin/env bash
# Stamp apps/gui-flutter/pubspec.yaml version from a release tag.
# Usage: stamp-release-version.sh <tag> [build_number] [pubspec_path]
# Prints VERSION_NAME=… and VERSION_BUILD=… (GitHub Actions $GITHUB_OUTPUT friendly).
set -euo pipefail

TAG="${1:-}"
BUILD="${2:-1}"
PUBSPEC="${3:-apps/gui-flutter/pubspec.yaml}"

if [[ -z "$TAG" ]]; then
  echo "usage: $0 <tag> [build_number] [pubspec_path]" >&2
  exit 1
fi

if [[ ! "$BUILD" =~ ^[0-9]+$ ]]; then
  echo "invalid build number: $BUILD" >&2
  exit 1
fi

# v1.2.3 or v1.2.3-beta.1 — prerelease ids must be non-empty and dot-separated
if [[ ! "$TAG" =~ ^v([0-9]+\.[0-9]+\.[0-9]+)(-[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*)?$ ]]; then
  echo "invalid tag (want vMAJOR.MINOR.PATCH[-prerelease]): $TAG" >&2
  exit 1
fi

CORE="${BASH_REMATCH[1]}"
PRE="${BASH_REMATCH[2]:-}"
NAME="${CORE}${PRE}"
VERSION_LINE="version: ${NAME}+${BUILD}"

if [[ ! -f "$PUBSPEC" ]]; then
  echo "pubspec not found: $PUBSPEC" >&2
  exit 1
fi

# Portable in-place edit (GNU sed -i vs BSD sed -i '')
tmp="$(mktemp)"
awk -v line="$VERSION_LINE" '
  /^version:/ { print line; next }
  { print }
' "$PUBSPEC" >"$tmp"
mv "$tmp" "$PUBSPEC"

echo "VERSION_NAME=$NAME"
echo "VERSION_BUILD=$BUILD"
