#!/usr/bin/env bash
# Fail on PRs that change a shipped MIDI mapping bundle under mappings/
# unless that bundle's mapping_version is strictly newer than the base ref.
# Local: MOON_BASE=$(git merge-base HEAD origin/main) scripts/check-mapping-version-bump.sh
# Or pass the base SHA as $1.
set -euo pipefail

BASE_SHA="${1:-${MOON_BASE:-${GITHUB_BASE_SHA:-}}}"
if [[ -z "${BASE_SHA}" ]]; then
  echo "error: base SHA required (pass arg, MOON_BASE, or GITHUB_BASE_SHA)" >&2
  exit 2
fi

if ! git cat-file -e "${BASE_SHA}^{commit}" 2>/dev/null; then
  echo "error: base SHA ${BASE_SHA} is unreachable; fetch the PR base before running" >&2
  exit 2
fi

# Globals filled by parse_version: _pv_core (numeric array), _pv_release (0|1).
_pv_core=()
_pv_release=0

# Mirror crates/controller/src/engine.rs parse_version (~L1174).
parse_version() {
  local version="$1"
  version="${version#"${version%%[![:space:]]*}"}"
  version="${version%"${version##*[![:space:]]}"}"
  # Match Rust strip_prefix('v') — uppercase V stays and fails numeric parse.
  if [[ "${version}" == v* ]]; then
    version="${version:1}"
  fi
  if [[ -z "${version}" ]]; then
    return 1
  fi

  local without_build="${version%%+*}"
  local core release=1
  if [[ "${without_build}" == *-* ]]; then
    local pre="${without_build#*-}"
    if [[ -n "${pre}" ]]; then
      core="${without_build%%-*}"
      release=0
    else
      core="${without_build}"
    fi
  else
    core="${without_build}"
  fi

  if [[ -z "${core}" ]]; then
    return 1
  fi

  local -a parts=()
  local IFS='.'
  read -ra parts <<<"${core}"
  unset IFS

  _pv_core=()
  local part
  for part in "${parts[@]}"; do
    if [[ ! "${part}" =~ ^[0-9]+$ ]]; then
      return 1
    fi
    _pv_core+=("$((10#${part}))")
  done

  while [[ ${#_pv_core[@]} -gt 1 && ${_pv_core[-1]} -eq 0 ]]; do
    unset '_pv_core[-1]'
  done

  _pv_release="${release}"
  return 0
}

# True iff head parses as strictly newer than base (both must already parse).
version_is_newer() {
  local head_raw="$1" base_raw="$2"
  local -a head_core base_core
  local head_release base_release

  parse_version "${head_raw}" || return 1
  head_core=("${_pv_core[@]}")
  head_release="${_pv_release}"

  parse_version "${base_raw}" || return 1
  base_core=("${_pv_core[@]}")
  base_release="${_pv_release}"

  local i min_len=${#head_core[@]}
  if ((${#base_core[@]} < min_len)); then
    min_len=${#base_core[@]}
  fi
  for ((i = 0; i < min_len; i++)); do
    if ((head_core[i] > base_core[i])); then
      return 0
    fi
    if ((head_core[i] < base_core[i])); then
      return 1
    fi
  done
  if ((${#head_core[@]} > ${#base_core[@]})); then
    return 0
  fi
  if ((${#head_core[@]} < ${#base_core[@]})); then
    return 1
  fi
  if ((head_release > base_release)); then
    return 0
  fi
  return 1
}

# Extract mapping_version from device.toml text at ref (HEAD = filesystem).
read_mapping_version() {
  local ref="$1" bundle_id="$2" text=""
  if [[ "${ref}" == "HEAD" ]]; then
    if [[ ! -f "mappings/${bundle_id}/device.toml" ]]; then
      printf '%s\n' ""
      return
    fi
    text="$(<"mappings/${bundle_id}/device.toml")"
  else
    if ! text="$(git show "${ref}:mappings/${bundle_id}/device.toml" 2>/dev/null)"; then
      printf '%s\n' ""
      return
    fi
  fi

  local line
  while IFS= read -r line || [[ -n "${line}" ]]; do
    # Basic TOML double-quoted string only (no escapes); matches toml::Value::as_str shape.
    if [[ "${line}" =~ ^[[:space:]]*mapping_version[[:space:]]*=[[:space:]]*\"([^\"\\]*)\" ]]; then
      printf '%s\n' "${BASH_REMATCH[1]}"
      return
    fi
  done <<<"${text}"
  printf '%s\n' ""
}

fmt_ver() {
  local raw="$1"
  if [[ -z "${raw}" ]]; then
    printf '%s' "missing"
  elif parse_version "${raw}"; then
    printf '%s' "${raw}"
  else
    printf '%s' "unparseable(${raw})"
  fi
}

if ! diff_out="$(git diff --name-only --diff-filter=ACMR "${BASE_SHA}" HEAD -- mappings/)"; then
  echo "error: git diff failed against base ${BASE_SHA}" >&2
  exit 2
fi
changed_paths=()
if [[ -n "${diff_out}" ]]; then
  mapfile -t changed_paths <<<"${diff_out}"
fi

declare -A bundle_ids=()
for path in "${changed_paths[@]+"${changed_paths[@]}"}"; do
  if [[ "${path}" =~ ^mappings/([^/]+)/ ]]; then
    bundle_ids["${BASH_REMATCH[1]}"]=1
  fi
done

if [[ ${#bundle_ids[@]} -eq 0 ]]; then
  exit 0
fi

failed=0
for bundle_id in "${!bundle_ids[@]}"; do
  if [[ ! -f "mappings/${bundle_id}/device.toml" ]]; then
    # Full delete (or device.toml removed); --diff-filter already dropped pure D paths.
    continue
  fi

  head_raw="$(read_mapping_version HEAD "${bundle_id}")"
  base_exists=0
  if git cat-file -e "${BASE_SHA}:mappings/${bundle_id}/device.toml" 2>/dev/null; then
    base_exists=1
  fi

  if [[ "${base_exists}" -eq 0 ]]; then
    # New bundle: HEAD mapping_version must parse.
    if parse_version "${head_raw}"; then
      continue
    fi
    echo "error: mappings/${bundle_id}: mapping_version must be set on new bundles (base=missing head=$(fmt_ver "${head_raw}"))" >&2
    failed=1
    continue
  fi

  base_raw="$(read_mapping_version "${BASE_SHA}" "${bundle_id}")"

  if [[ -z "${base_raw}" ]]; then
    # Existing; base version empty — first parseable HEAD version counts as increase.
    if parse_version "${head_raw}"; then
      continue
    fi
    echo "error: mappings/${bundle_id}: mapping_version must increase on PRs that change the bundle (base=missing head=$(fmt_ver "${head_raw}"))" >&2
    failed=1
    continue
  fi

  if ! parse_version "${base_raw}"; then
    echo "error: mappings/${bundle_id}: mapping_version must increase on PRs that change the bundle (base=unparseable(${base_raw}) head=$(fmt_ver "${head_raw}")); set a parseable newer version on HEAD" >&2
    failed=1
    continue
  fi

  if ! parse_version "${head_raw}"; then
    echo "error: mappings/${bundle_id}: mapping_version must increase on PRs that change the bundle (base=${base_raw} head=$(fmt_ver "${head_raw}"))" >&2
    failed=1
    continue
  fi

  if version_is_newer "${head_raw}" "${base_raw}"; then
    continue
  fi

  echo "error: mappings/${bundle_id}: mapping_version must increase on PRs that change the bundle (base=${base_raw} head=${head_raw})" >&2
  failed=1
done

if [[ "${failed}" -ne 0 ]]; then
  exit 1
fi
exit 0
