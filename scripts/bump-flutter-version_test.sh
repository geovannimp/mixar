#!/usr/bin/env bash
# Self-check for scripts/bump-flutter-version.sh (no frameworks).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SCRIPT="$ROOT/scripts/bump-flutter-version.sh"
FIXTURE="$(mktemp)"
trap 'rm -f "$FIXTURE"' EXIT

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

write_ver() {
  cat >"$FIXTURE" <<EOF
name: gui_flutter
version: $1
description: test
EOF
}

"$SCRIPT" >/dev/null 2>&1 && fail "expected missing bump to fail"
"$SCRIPT" "nope" "$FIXTURE" >/dev/null 2>&1 && fail "expected invalid bump to fail"

write_ver "1.2.3+9"
out="$("$SCRIPT" patch "$FIXTURE")"
grep -q '^version: 1\.2\.4+1$' "$FIXTURE" || fail "patch: $(cat "$FIXTURE")"
echo "$out" | grep -qx 'TAG=v1.2.4' || fail "patch TAG: $out"
echo "$out" | grep -qx 'OLD_VERSION=1.2.3+9' || fail "patch OLD: $out"

write_ver "1.2.3+1"
out="$("$SCRIPT" minor "$FIXTURE")"
grep -q '^version: 1\.3\.0+1$' "$FIXTURE" || fail "minor: $(cat "$FIXTURE")"
echo "$out" | grep -qx 'TAG=v1.3.0' || fail "minor TAG: $out"

write_ver "1.2.3-beta.1+4"
out="$("$SCRIPT" major "$FIXTURE")"
grep -q '^version: 2\.0\.0+1$' "$FIXTURE" || fail "major from prerelease: $(cat "$FIXTURE")"
echo "$out" | grep -qx 'TAG=v2.0.0' || fail "major TAG: $out"

echo "bump-flutter-version_test: ok"
