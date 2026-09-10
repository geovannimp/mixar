#!/usr/bin/env bash
# Bump apps/gui-flutter/pubspec.yaml semver (major|minor|patch).
# Usage: bump-flutter-version.sh <major|minor|patch> [pubspec_path]
# Prints VERSION_NAME=… VERSION_BUILD=… TAG=… OLD_VERSION=…
set -euo pipefail

BUMP="${1:-}"
PUBSPEC="${2:-apps/gui-flutter/pubspec.yaml}"

usage() {
  echo "usage: $0 <major|minor|patch> [pubspec_path]" >&2
}

if [[ -z "$BUMP" ]]; then
  usage
  exit 1
fi

case "$BUMP" in
  major|minor|patch) ;;
  *)
    echo "invalid bump (want major|minor|patch): $BUMP" >&2
    exit 1
    ;;
esac

if [[ ! -f "$PUBSPEC" ]]; then
  echo "pubspec not found: $PUBSPEC" >&2
  exit 1
fi

line="$(grep -E '^version:' "$PUBSPEC" | head -n1 || true)"
if [[ -z "$line" ]]; then
  echo "no version: line in $PUBSPEC" >&2
  exit 1
fi

# version: 1.2.3+4  or  1.2.3-beta.1+4  → bump uses core MAJOR.MINOR.PATCH only
rest="${line#version:}"
rest="${rest#"${rest%%[![:space:]]*}"}"
OLD_VERSION="$rest"

if [[ ! "$rest" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)([-+].*)?$ ]]; then
  echo "unparseable version line: $line" >&2
  exit 1
fi

MAJOR="${BASH_REMATCH[1]}"
MINOR="${BASH_REMATCH[2]}"
PATCH="${BASH_REMATCH[3]}"

case "$BUMP" in
  major)
    MAJOR=$((MAJOR + 1))
    MINOR=0
    PATCH=0
    ;;
  minor)
    MINOR=$((MINOR + 1))
    PATCH=0
    ;;
  patch)
    PATCH=$((PATCH + 1))
    ;;
esac

# Fresh build number on each marketed version bump.
NAME="${MAJOR}.${MINOR}.${PATCH}"
BUILD=1
VERSION_LINE="version: ${NAME}+${BUILD}"

tmp="$(mktemp)"
awk -v line="$VERSION_LINE" '
  /^version:/ { print line; next }
  { print }
' "$PUBSPEC" >"$tmp"
mv "$tmp" "$PUBSPEC"

echo "OLD_VERSION=$OLD_VERSION"
echo "VERSION_NAME=$NAME"
echo "VERSION_BUILD=$BUILD"
echo "TAG=v${NAME}"
