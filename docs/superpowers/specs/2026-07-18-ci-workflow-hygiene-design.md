# CI Workflow Hygiene Design

**Issue:** [#74](https://github.com/geovannimp/rust-dj-engine/issues/74)  
**Date:** 2026-07-18

## Goal

Make GitHub Actions green and trustworthy: install native deps, enforce stable lint/test/audit as separate workflows, drop docs for now, and run beta as a daily warning-only check on `main`.

## Decisions

| Topic | Choice |
|-------|--------|
| Workflow layout | Concern-per-file (lint / test / audit / rust-beta-audit) |
| Required gate | Stable only |
| Beta | Daily cron on `main`, warning-only (`continue-on-error`), file `rust-beta-audit.yml` |
| Nightly | Dropped |
| Docs / Pages | Disabled for now |
| Audit | Hybrid: fail on vulnerabilities; warnings are informational; add `crates/audit.toml` only when ignores are needed |
| Compile runner | Direct `cargo` in `crates/` (not a single `moon ci` Actions job) |
| Cache | `actions/cache@v4` |
| System deps | `pkg-config`, `libasound2-dev`, `libpipewire-0.3-dev` before any compile |

## Workflow layout

Replace monolithic `.github/workflows/ci.yml` with:

| File | Purpose | Triggers | Blocks PRs? |
|------|---------|----------|-------------|
| `lint.yml` | stable `cargo fmt --check` + `clippy -D warnings` | PR + push to `main`/`develop` | Yes |
| `test.yml` | stable `cargo test --release` | PR + push to `main`/`develop` | Yes |
| `audit.yml` | hybrid `cargo audit` | PR + push to `main`/`develop` | Yes |
| `rust-beta-audit.yml` | beta fmt/clippy/test | daily cron on `main` + `workflow_dispatch` | No |

Shared apt install step (inline or composite action) runs before clippy/test/beta compile steps.

## System dependencies

Ubuntu runners must install before compiling crates that pull CPAL / PipeWire:

```bash
sudo apt-get update
sudo apt-get install -y pkg-config libasound2-dev libpipewire-0.3-dev
```

Add further packages only if a clean CI build still fails on a missing `.pc`.

## Audit policy

1. Prefer clearing vulns by bumping deps (`bytes` ≥ 1.11.1, `crossbeam-epoch` ≥ 0.9.20 when present).
2. Warnings (unmaintained / unsound / yanked) are informational and do not fail CI (`cargo audit` default).
3. Add `crates/audit.toml` only when a vulnerability ignore is required; document rationale next to each ID.
4. `audit.yml` fails the job when unresolved vulnerabilities remain. Policy note lives on the workflow.

## Out of scope

- Re-enabling rustdoc / GitHub Pages
- Nightly toolchain matrix
- Reworking moon task graphs beyond keeping local `moon` usable
- Changing GitHub branch-protection UI (PR notes which checks to require: lint, test, audit)

## Acceptance

- Fresh PR against `main` gets green lint, test, and audit without local-only hacks
- Native deps installed before compile jobs
- `cargo fmt --check` and clippy pass on a clean tree
- Audit policy documented (this spec + `audit.yml` comment)
- No duplicate fmt/clippy across a beta/nightly matrix; beta is daily warning-only only
