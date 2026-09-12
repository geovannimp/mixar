#!/usr/bin/env bash
# AppImage AppRun execs ./gui_flutter; bundle BINARY_NAME is Mixar — alias must resolve.
set -euo pipefail
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
: >"$tmp/Mixar"
(cd "$tmp" && cmake -E create_symlink Mixar gui_flutter)
test -L "$tmp/gui_flutter"
test "$(readlink "$tmp/gui_flutter")" = "Mixar"
echo "ok: gui_flutter -> Mixar"
