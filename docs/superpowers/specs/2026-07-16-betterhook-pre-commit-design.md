# Betterhook Pre-commit (Rustfmt Autofix) Design

**Issue:** [#73](https://github.com/geovannimp/rust-dj-engine/issues/73)  
**Date:** 2026-07-16

## Goal

Prevent unformatted Rust from landing in commits by auto-running `cargo fmt` with restage on pre-commit, bootstrapped on the first local `cargo build` with no manual install step.

## Requirements

| Decision | Choice |
|----------|--------|
| Hook manager | [betterhook](https://crates.io/crates/betterhook-cli) (`stage_fixed`) |
| Default hook | `cargo fmt` only (no clippy) |
| Autofix | `stage_fixed = true` (format + restage; commit continues) |
| Bootstrap | Tiny `hooks-install` crate `build.rs` installs pinned `betterhook-cli` into `tools/` and runs `betterhook install` |
| Pin | `betterhook-cli` **0.1.0** |
| CI | Skip bootstrap when `CI` is set; CI keeps `cargo fmt -- --check` |
| Skip (emergency) | `BETTERHOOK_SKIP=cargo-fmt`, `BETTERHOOK_BOOTSTRAP=0`, or `git commit --no-verify` |

Out of scope: clippy in the hook, pre-push suite, vendoring Go lefthook binaries.

## Architecture

```text
First local cargo build (pulls audio-core → hooks-install)
  → build.rs installs tools/bin/betterhook (pinned)
  → betterhook install --no-unit
  → writes .git/hooks/pre-commit (absolute path to tools/bin/betterhook)

git commit (staged *.rs)
  → cargo fmt --
  → stage_fixed restages formatted files
  → commit proceeds
```

## Components

### `betterhook.toml`

Pre-commit job `cargo-fmt`: `run = "rustfmt --edition 2021 {staged_files}"`, `glob = ["*.rs"]`, `stage_fixed = true`, `stash_untracked = false`.

Uses `rustfmt` on staged paths (not `cargo fmt --`) so formatting still works when `Cargo.toml` workspace membership differs from the index. Disables betterhook’s worktree stash to avoid stash/pop conflicts with in-progress edits.

### `hooks-install`

Workspace crate whose `build.rs`:

1. No-op if `CI` set, `BETTERHOOK_BOOTSTRAP=0`, or no `.git`.
2. No-op if marker matches pinned version and `tools/bin/betterhook` exists.
3. Else `cargo install betterhook-cli --version <pin> --root tools/ --locked --force`.
4. Run `tools/bin/betterhook install --no-unit`.
5. Write marker (version string). On failure: `cargo:warning`, do not fail the build.

Pulled in via `audio-core` `[build-dependencies]` so normal engine builds trigger bootstrap.

### `tools/`

Gitignored local install root (`tools/bin/betterhook`).

### Docs

README + AGENTS.md: first build installs hooks; skip env vars; prefer not using `--no-verify`.

## Acceptance

1. Hook config + bootstrap live in-repo and are documented.
2. Unformatted Rust is fixed and restaged on commit.
3. Commits do not run network installs or full `cargo test` / clippy.
4. Skip / emergency paths are documented.
5. CI bootstrap is a no-op; CI fmt check still gates PRs.
