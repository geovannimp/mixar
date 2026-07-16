# Lefthook Pre-commit (Rustfmt Autofix) Design

**Issue:** [#73](https://github.com/geovannimp/rust-dj-engine/issues/73)  
**Date:** 2026-07-16  
**Updated:** 2026-07-16 — switched from betterhook/`hooks-install` to npm workspaces + lefthook

## Goal

Prevent unformatted Rust from landing in commits by auto-running `rustfmt` with restage on pre-commit, bootstrapped via the same Node toolchain already required for `gui-app`.

## Requirements

| Decision | Choice |
|----------|--------|
| Hook manager | [lefthook](https://lefthook.dev) (`stage_fixed`) |
| Default hook | `rustfmt` on staged `*.rs` only (no clippy) |
| Autofix | `stage_fixed: true` (format + restage; commit continues) |
| Bootstrap | Root `package.json` `prepare` → `lefthook install` after `npm install` |
| Monorepo | npm workspaces with `gui-app` as the workspace package |
| CI | Unchanged `cargo fmt -- --check`; hooks not required in CI |
| Skip (emergency) | `LEFTHOOK_EXCLUDE=cargo-fmt`, `LEFTHOOK=0`, or `git commit --no-verify` |

Out of scope: clippy in the hook, cargo `build.rs` bootstrap, vendoring binaries.

## Architecture

```text
npm install (repo root)
  → installs lefthook (devDependency)
  → prepare runs `lefthook install`
  → writes .git/hooks/pre-commit

git commit (staged *.rs)
  → rustfmt --edition 2021 {staged_files}
  → stage_fixed restages formatted files
  → commit proceeds
```

## Components

### Root `package.json`

- `workspaces: ["gui-app"]`
- `devDependencies.lefthook`
- `prepare`: `lefthook install`
- Convenience scripts: `dev:gui`, `build:gui`, `tauri`

### `lefthook.yml`

Pre-commit command `cargo-fmt`: `rustfmt --edition 2021 {staged_files}`, `glob: "*.rs"`, `stage_fixed: true`.

Uses `rustfmt` on staged paths (not `cargo fmt --`) so formatting still works when `Cargo.toml` workspace membership differs from the index.

### Docs

README + AGENTS.md: run `npm install` once at repo root; skip env vars; prefer not using `--no-verify`.

## Acceptance

1. Hook config + root npm workspace live in-repo and are documented.
2. Unformatted Rust is fixed and restaged on commit.
3. Commits do not run network installs or full `cargo test` / clippy.
4. Skip / emergency paths are documented.
5. CI fmt check still gates PRs.
