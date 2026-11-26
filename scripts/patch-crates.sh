#!/usr/bin/env bash
# Materialize the patches in crates/patches/ into crates/target/patch.
#
# Cargo resolves `[patch.crates-io]` before any build script runs, so this must
# run before the first cargo/flutter build and again after `cargo clean`.
# `mise install` runs it through the `rust-patch` task.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
crates="$root/crates"
crate_name="cpal"
crate_version="0.18.2"
patch_file="$crates/patches/${crate_name}+${crate_version}.patch"
target="$crates/target/patch/${crate_name}-${crate_version}"
marker="$target/.patch-inputs"

if [[ ! -f "$patch_file" ]]; then
  echo "error: missing patch file $patch_file" >&2
  exit 1
fi

inputs_hash() {
  (cd "$crates" && cksum patches/*.patch Cargo.toml)
}

valid_target() {
  [[ -f "$target/Cargo.toml" && -f "$target/src/lib.rs" ]] &&
    grep -q "^name = \"${crate_name}\"" "$target/Cargo.toml" &&
    grep -q 'RtPromoterHandle' "$target/src/host/pipewire/rt_promote.rs" &&
    grep -q 'promoter.demote()' "$target/src/host/pipewire/device.rs" &&
    grep -q 'rt_promoter_handle' "$target/src/host/pipewire/stream.rs"
}

# The marker hashes every patch input, so adding/editing any patch or changing
# the workspace manifest rebuilds the tree. CI also always rebuilds: rust-cache
# can restore a partial crates/target/patch over the one mise installed.
if valid_target && [[ -f "$marker" ]] &&
  [[ "$(inputs_hash)" == "$(cat "$marker")" ]] && [[ -z "${CI:-}" ]]; then
  exit 0
fi

if ! command -v cargo-patch-crate >/dev/null 2>&1; then
  echo "error: cargo-patch-crate is not installed." >&2
  echo "Run 'mise install', or install it with 'cargo install patch-crate --locked'." >&2
  exit 1
fi

args=()
if [[ -d "$target" ]]; then
  args+=(--force)
fi

cd "$crates"
cargo patch-crate "${args[@]}"
inputs_hash > "$marker"
