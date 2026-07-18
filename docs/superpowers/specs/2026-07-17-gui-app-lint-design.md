# gui-app Frontend Lint / Format Design

**Issue:** [#76](https://github.com/geovannimp/rust-dj-engine/issues/76)  
**Date:** 2026-07-17

## Goal

Add linting and formatting for `gui-app` (TypeScript + React) and wire it into the same lefthook pre-commit + CI lint flow used for Rust, without slowing commits (staged files only) or introducing a separate CI job yet.

## Requirements

| Decision | Choice |
|----------|--------|
| Toolchain | [Oxlint](https://oxc.rs/docs/guide/usage/linter) + [Oxfmt](https://oxc.rs/docs/guide/usage/formatter) |
| Package ownership | DevDeps + scripts + config live in `gui-app` (workspace package) |
| Scripts | `lint`, `lint:fix`, `format`, `format:check` in `gui-app/package.json` |
| Baseline | Format + autofix the whole `gui-app` TS/TSX tree on first land |
| Pre-commit | Lefthook jobs on staged `*.{ts,tsx}` with `stage_fixed: true` |
| CI | Extend existing `lint` job (Node setup + npm ci + format check + lint) |
| Typecheck in hook/CI lint | Out of scope (`tsc` stays on build path) |
| Future monorepo runner | Deferred; keep tooling local to `gui-app` so a later orchestrator can call workspace scripts |
| Node version | Pin Node 22 via root `.nvmrc` (and CI `node-version-file`) for stable oxfmt worker teardown |

Out of scope: ESLint/Prettier, CSS/JSON formatting beyond what Oxfmt touches by default for TS/TSX, full `tsc` in the lint job, a dedicated `frontend-lint` CI job.

## Architecture

```text
gui-app/
  package.json          # oxlint, oxfmt; lint / format scripts
  .oxlintrc.json        # minimal ignores (dist, node_modules, …)
  .oxfmtrc.json         # minimal ignores (same)

lefthook.yml
  cargo-fmt             # existing
  oxfmt                 # staged *.{ts,tsx} → format + stage_fixed
  oxlint                # staged *.{ts,tsx} → oxlint --fix + stage_fixed

.github/workflows/ci.yml  (lint job)
  rustfmt + clippy      # existing
  setup-node + npm ci
  npm run format:check -w gui-app
  npm run lint -w gui-app
```

## Components

### `gui-app` scripts

| Script | Command | Purpose |
|--------|---------|---------|
| `lint` | `oxlint .` | Fail on lint errors (CI + local) |
| `lint:fix` | `oxlint --fix .` | Autofix where safe |
| `format` | `oxfmt .` | Write formatting |
| `format:check` | `oxfmt --check .` | CI / dry-run format gate |

### Lefthook

Two new pre-commit commands (names: `oxfmt`, `oxlint`), glob `*.{ts,tsx}`, run via workspace binaries so PATH resolves after `npm install`:

- Format staged files, then lint-with-fix staged files
- `stage_fixed: true` on both
- Skip: `LEFTHOOK_EXCLUDE=oxfmt` / `oxlint` (existing `LEFTHOOK=0` / `--no-verify` still apply)

### CI

Extend `jobs.lint` only: after Rust steps, install Node (LTS), `npm ci` at repo root, then `format:check` and `lint` for workspace `gui-app`.

## Error handling

- Pre-commit: if oxlint reports unfixable errors after `--fix`, the commit fails (same as a failed hook command).
- CI: either format drift or lint errors fail the `lint` job.
- Missing `node_modules`: `npm install` at root is already required for lefthook; CI runs `npm ci`.

## Testing / verification

1. `npm run format -w gui-app` then `npm run format:check -w gui-app` exits 0.
2. `npm run lint:fix -w gui-app` then `npm run lint -w gui-app` exits 0.
3. Stage an intentionally ugly `.tsx` file; commit restages a formatted/fixed version (or fails on unfixable lint).
4. CI `lint` job includes the new Node steps.

## Acceptance

1. `gui-app` has documented `lint` / `format` (and check/fix) scripts; `lint` fails on errors.
2. Staged `*.ts` / `*.tsx` are formatted and lint-fixed via lefthook with `stage_fixed`.
3. CI `lint` job runs frontend format check + lint.
4. Pre-commit stays staged-files-only (no full typecheck).
