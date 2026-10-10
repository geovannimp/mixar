#!/usr/bin/env bash
# Fail unless flutter_rust_bridge_codegen matches the Dart/Rust pin (2.13.0),
# then run `flutter_rust_bridge_codegen` with the given args (default: generate).
set -euo pipefail

EXPECTED="${FRB_CODEGEN_VERSION:-2.13.0}"
export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"

install_hint() {
  echo "  cargo install flutter_rust_bridge_codegen --version ${EXPECTED} --locked" >&2
}

if ! command -v flutter_rust_bridge_codegen >/dev/null 2>&1; then
  echo "error: flutter_rust_bridge_codegen not found (need ${EXPECTED}). Install with:" >&2
  install_hint
  exit 1
fi

# e.g. "flutter_rust_bridge_codegen 2.13.0"
actual="$(flutter_rust_bridge_codegen --version | awk '{print $NF}')"
if [[ "${actual}" != "${EXPECTED}" ]]; then
  echo "error: flutter_rust_bridge_codegen ${actual} != required ${EXPECTED}. Reinstall with:" >&2
  install_hint
  exit 1
fi

if [[ "${#}" -eq 0 ]]; then
  set -- generate
fi

exec flutter_rust_bridge_codegen "${@}"
