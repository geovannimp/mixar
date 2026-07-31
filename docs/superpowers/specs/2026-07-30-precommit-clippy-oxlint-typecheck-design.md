# Pre-commit clippy + oxlint typeCheck

Date: 2026-07-30  
Issue: [#99](https://github.com/geovannimp/rust-dj-engine/issues/99)  
Depends: [#115](https://github.com/geovannimp/rust-dj-engine/issues/115) / [#116](https://github.com/geovannimp/rust-dj-engine/pull/116) (TypeScript 7)  
Status: accepted

## Goal

Close the gap between local pre-commit (`format:staged` / `lint:staged`) and CI lint/build so clippy warnings and TypeScript type errors fail the commit hook, without running full-workspace CI on every commit.

## Design

### Rust (`crates` moon project)

- Add `lint-files` moon task (pre-commit only, `runInCI: false`).
- When any staged `**/*.rs` exists under `crates/`, run the same clippy invocation as CI:

  `cargo clippy --all-targets --all-features -- -D warnings`

- Use moon `affectedFiles` with `pass: false` (clippy does not take file path args) and `passDotWhenNoResults: false` so the task skips when no `.rs` is staged.
- CI `lint` remains the same full-workspace clippy command.

### TypeScript (`gui-app`)

- Add `oxlint-tsgolint`.
- Enable `options.typeAware` and `options.typeCheck` in `oxlint.config.ts`.
- Keep `lint` / `lint:fix` as `oxlint` / `oxlint --fix`; type diagnostics piggyback.
- CI `build` may keep `tsc && vite build` for emit/bundler; dropping redundant `tsc` is a follow-up.

### Lefthook

- Remove the lint job’s `*.{ts,tsx}`-only glob so Rust `lint-files` runs when `.rs` is staged.
- Keep single `format` / `lint` jobs calling moon staged scripts (no parallel hook commands).

## Out of scope

- `apps/gui-app/src-tauri` clippy on commit (outside `crates` moon project).
- Required GitHub checks (#85).
- Replacing CI `tsc` in `gui-app:build`.

## Acceptance

- [x] Staged Rust change that triggers a clippy `-D warnings` failure fails `npm run lint:staged`.
- [x] Staged gui-app TS type error fails `npm run lint:staged` via oxlint typeCheck.
- [x] Hook still skips Rust clippy when no `.rs` staged; TS oxlint when no gui-app TS affected.
- [x] CI full-package `:lint` / `:build` unchanged in intent.
