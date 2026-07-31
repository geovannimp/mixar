# Pre-commit clippy + oxlint typeCheck Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire affected-crate clippy and oxlint typeCheck into pre-commit `lint:staged` so #99 class of CI failures fail locally.

**Architecture:** Moon `lint-files` for `crates` maps staged `.rs` → `cargo clippy -p …`. Gui-app enables oxlint `typeAware`/`typeCheck` via `oxlint-tsgolint`. Lefthook lint glob widened so Rust tasks run.

**Tech Stack:** moon, lefthook, cargo clippy, oxlint + oxlint-tsgolint.

**Spec:** `docs/superpowers/specs/2026-07-30-precommit-clippy-oxlint-typecheck-design.md`

## Global Constraints

- Clippy on affected packages only (not full workspace) for `lint-files`.
- Same clippy deny flags as CI for those packages: `--all-targets` and `-D warnings`. Prefer also `--all-features` to match CI `lint` when clipping packages.
- Oxlint typeCheck (not separate `tsc --noEmit`) for the TS gate.
- Do not add src-tauri clippy in this slice.
- Prefer fewest files; reuse moon `affectedFiles` patterns from `format-files`.

## File map

| File | Role |
|------|------|
| `crates/scripts/clippy-packages.sh` | Map `.rs` paths → unique `-p` names; run clippy |
| `crates/moon.yml` | Add `lint-files` calling the script |
| `apps/gui-app/package.json` | Add `oxlint-tsgolint` |
| `apps/gui-app/oxlint.config.ts` | `typeAware` + `typeCheck` |
| `lefthook.yml` | Drop TS-only lint glob |
| `package-lock.json` | Lockfile for new dep |
| Spec | Status accepted when done |

---

### Task 1: Rust lint-files + clippy script

**Files:**
- Create: `crates/scripts/clippy-packages.sh`
- Modify: `crates/moon.yml`

- [ ] **Step 1: Add script**

Script under `crates/` (moon project root):

```bash
#!/usr/bin/env bash
set -euo pipefail
# Args: paths relative to crates/ (or absolute). Collect unique top-level package dirs.
pkgs=()
for f in "$@"; do
  rel="${f#./}"
  case "$rel" in
    *.rs) ;;
    *) continue ;;
  esac
  pkg="${rel%%/*}"
  [[ -f "$pkg/Cargo.toml" ]] || continue
  pkgs+=("$pkg")
done
# unique
mapfile -t pkgs < <(printf '%s\n' "${pkgs[@]}" | sort -u)
[[ ${#pkgs[@]} -eq 0 ]] && exit 0
args=()
for p in "${pkgs[@]}"; do args+=(-p "$p"); done
exec cargo clippy "${args[@]}" --all-targets --all-features -- -D warnings
```

- [ ] **Step 2: moon lint-files**

Mirror `format-files` options; command runs the script with affected files as args.

- [ ] **Step 3: Smoke**

Stage a dummy path or run script with `engine-dsp/src/lib.rs` and confirm clippy starts for `-p engine-dsp`.

- [ ] **Step 4: Commit**

```
chore(crates): add lint-files clippy for staged packages
```

---

### Task 2: Oxlint typeCheck

**Files:**
- Modify: `apps/gui-app/package.json`, lockfile, `oxlint.config.ts`

- [ ] **Step 1: Install** `oxlint-tsgolint` in gui-app (from repo root npm workspace).

- [ ] **Step 2: Config**

```ts
options: {
  typeAware: true,
  typeCheck: true,
},
```

- [ ] **Step 3: Verify** `cd apps/gui-app && npx oxlint` exits 0 (or fix any new type-aware noise if trivial; escalate if large).

- [ ] **Step 4: Commit**

```
chore(gui-app): enable oxlint typeAware and typeCheck
```

---

### Task 3: Lefthook + docs

**Files:**
- Modify: `lefthook.yml`
- Modify: design spec status → accepted
- Optionally one-line AGENTS.md if it still claims hooks ≠ clippy/tsc incorrectly

- [ ] **Step 1: Remove lint `glob: "*.{ts,tsx}"`** so Rust lint-files participates.

- [ ] **Step 2: Mark spec accepted**

- [ ] **Step 3: Commit + open PR** linked to #99
