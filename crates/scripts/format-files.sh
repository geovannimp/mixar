#!/usr/bin/env bash
# Pre-commit helper for moon rust:format-files.
# Must be a real script (not `bash -c '… "$@" …'`): moon may wrap the task in
# another `bash -c`, which leaves $@ empty inside an inline -c body and skips
# rustfmt while still exiting 0.
set -euo pipefail
files=()
for f in "$@"; do
  if [[ -f "$f" ]]; then
    files+=("$f")
  fi
done
if [[ ${#files[@]} -eq 0 ]]; then
  exit 0
fi
exec rustfmt --edition 2021 "${files[@]}"
