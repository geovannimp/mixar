#!/usr/bin/env bash
# Self-check for scripts/stamp-release-version.sh (no frameworks).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SCRIPT="$ROOT/scripts/stamp-release-version.sh"
FIXTURE="$(mktemp)"
trap 'rm -f "$FIXTURE"' EXIT

cat >"$FIXTURE" <<'EOF'
name: gui_flutter
version: 0.0.0+0
description: test
EOF

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

"$SCRIPT" >/dev/null 2>&1 && fail "expected missing tag to fail"
"$SCRIPT" "1.0.0" 1 "$FIXTURE" >/dev/null 2>&1 && fail "expected tag without v to fail"
"$SCRIPT" "vabc" 1 "$FIXTURE" >/dev/null 2>&1 && fail "expected non-semver to fail"
"$SCRIPT" "v1.2.3" nope "$FIXTURE" >/dev/null 2>&1 && fail "expected non-numeric build to fail"
"$SCRIPT" "v1.2.3-beta." 1 "$FIXTURE" >/dev/null 2>&1 && fail "expected trailing-dot prerelease to fail"
"$SCRIPT" "v1.2.3-beta..1" 1 "$FIXTURE" >/dev/null 2>&1 && fail "expected empty prerelease segment to fail"

out="$("$SCRIPT" "v1.2.3" 42 "$FIXTURE")"
grep -q '^version: 1\.2\.3+42$' "$FIXTURE" || fail "stable stamp: $(cat "$FIXTURE")"
echo "$out" | grep -qx 'VERSION_NAME=1.2.3' || fail "stable VERSION_NAME: $out"
echo "$out" | grep -qx 'VERSION_BUILD=42' || fail "stable VERSION_BUILD: $out"

out="$("$SCRIPT" "v1.2.3-beta.1" 7 "$FIXTURE")"
grep -q '^version: 1\.2\.3-beta\.1+7$' "$FIXTURE" || fail "prerelease stamp: $(cat "$FIXTURE")"
echo "$out" | grep -qx 'VERSION_NAME=1.2.3-beta.1' || fail "prerelease VERSION_NAME: $out"

echo "stamp-release-version_test: ok"
