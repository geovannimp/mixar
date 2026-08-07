#!/usr/bin/env bash
# Pre-commit helper for moon rust:format-files.
# Must be a real script (not `bash -c '… "$@" …'`): moon may wrap the task in
# another `bash -c`, which leaves $@ empty inside an inline -c body and skips
# rustfmt while still exiting 0.
set -euo pipefail

if [[ "${1:-}" == "--self-check" ]]; then
  tmp=$(mktemp -d)
  trap 'rm -rf "$tmp"' EXIT
  mkdir -p "$tmp/bin"
  existing="$tmp/exists.rs"
  : >"$existing"
  missing="$tmp/missing.rs"
  args_out="$tmp/rustfmt-args"
  cat >"$tmp/bin/rustfmt" <<'EOF'
#!/usr/bin/env bash
printf '%s\n' "$@" >"$RUSTFMT_ARGS_OUT"
EOF
  chmod +x "$tmp/bin/rustfmt"
  RUSTFMT_ARGS_OUT="$args_out" PATH="$tmp/bin:$PATH" "$0" "$existing" "$missing"
  mapfile -t got <"$args_out"
  rs=()
  for a in "${got[@]}"; do
    if [[ "$a" == *.rs ]]; then
      rs+=("$a")
    fi
  done
  [[ ${#rs[@]} -eq 1 && "${rs[0]}" == "$existing" ]]
  exit 0
fi

files=()
for f in "$@"; do
  if [[ -f "$f" ]]; then
    files+=("$f")
  fi
done
if [[ ${#files[@]} -eq 0 ]]; then
  exit 0
fi
exec rustfmt --edition 2021 -- "${files[@]}"
