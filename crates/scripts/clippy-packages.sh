#!/usr/bin/env bash
# Map staged .rs paths (relative to crates/) to workspace packages and clippy them.
# Used by moon lint-files (lefthook → lint:staged). Empty / non-.rs args → no-op.
set -euo pipefail

pkgs=()
for f in "$@"; do
  rel="${f#./}"
  case "$rel" in
    *.rs) ;;
    *) continue ;;
  esac
  # Paths may be repo-absolute (…/crates/engine-dsp/…) or crates-relative.
  if [[ "$rel" == *"/crates/"* ]]; then
    rel="${rel##*/crates/}"
  elif [[ "$rel" == crates/* ]]; then
    rel="${rel#crates/}"
  fi
  pkg="${rel%%/*}"
  [[ -n "$pkg" && -f "$pkg/Cargo.toml" ]] || continue
  pkgs+=("$pkg")
done

if [[ ${#pkgs[@]} -eq 0 ]]; then
  exit 0
fi

mapfile -t pkgs < <(printf '%s\n' "${pkgs[@]}" | sort -u)

args=()
for p in "${pkgs[@]}"; do
  args+=(-p "$p")
done

exec cargo clippy "${args[@]}" --all-targets --all-features -- -D warnings
