#!/usr/bin/env bash
# Materialize the patches in crates/patches/ into crates/target/patch.
#
# Cargo resolves `[patch.crates-io]` before any build script runs, so this must
# run before the first cargo/flutter build and again after `cargo clean`.
# `mise install` runs it through the `rust-patch` task.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
crates="$root/crates"
patch_file="$crates/patches/cpal+0.18.2.patch"
target="$crates/target/patch/cpal-0.18.2"

args=()
valid_target=0
if [[ -f "$target/Cargo.toml" && -f "$target/src/lib.rs" ]] &&
  grep -q '^name = "cpal"' "$target/Cargo.toml" &&
  grep -q 'RtPromoterHandle' "$target/src/host/pipewire/rt_promote.rs"; then
  valid_target=1
fi

if ((valid_target)) && [[ ! "$patch_file" -nt "$target/.cargo-ok" ]]; then
  exit 0
fi

# Missing, incomplete, stale, or unpatched target: rebuild it from the
# registry source. CI restores `target` from a cache that may hold a partial
# `target/patch`, so validate the package manifest before trusting it.
if [[ -d "$target" ]]; then
  args+=(--force)
fi

if ! command -v cargo-patch-crate >/dev/null 2>&1; then
  echo "error: cargo-patch-crate is not installed." >&2
  echo "Install it with 'cargo install patch-crate --version 0.1.13 --locked'" >&2
  echo "or run 'mise install'." >&2
  exit 1
fi

cd "$crates"
cargo patch-crate "${args[@]}"
