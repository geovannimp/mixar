# CI Workflow Hygiene Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Split failing monolithic CI into green stable lint/test/audit workflows, with daily warning-only beta and hybrid cargo-audit policy.

**Architecture:** Delete `.github/workflows/ci.yml`. Add four focused workflows under `.github/workflows/`, a small composite action for apt native deps, and `crates/audit.toml`. Prefer bumping vulnerable crates; allow warnings with documented ignores.

**Tech Stack:** GitHub Actions, Ubuntu runners, stable/beta Rust via `dtolnay/rust-toolchain`, `cargo fmt` / `clippy` / `test` / `audit`, `actions/cache@v4`.

## Global Constraints

- Working directory for Cargo commands: `crates/`
- Required PR checks: lint, test, audit on **stable** only
- Beta: `rust-beta-audit.yml`, daily cron, `continue-on-error: true`, never required
- Docs / Pages: not reintroduced
- Native deps before compile: `pkg-config libasound2-dev libpipewire-0.3-dev`
- Audit: fail on vulnerabilities; warnings allowed via `crates/audit.toml`
- Cache action: `actions/cache@v4`

## File map

| Path | Role |
|------|------|
| `.github/actions/install-native-deps/action.yml` | Composite: apt install audio build deps |
| `.github/workflows/lint.yml` | Stable fmt + clippy |
| `.github/workflows/test.yml` | Stable release tests |
| `.github/workflows/audit.yml` | Hybrid cargo-audit |
| `.github/workflows/rust-beta-audit.yml` | Daily beta warning-only |
| `.github/workflows/ci.yml` | Delete |
| `crates/audit.toml` | Advisory ignore / warning policy |
| `crates/Cargo.lock` | Bump vulnerable transitive deps if needed |

---

### Task 1: Shared native-deps action + delete old CI

**Files:**
- Create: `.github/actions/install-native-deps/action.yml`
- Delete: `.github/workflows/ci.yml`

**Interfaces:**
- Produces: composite action `./.github/actions/install-native-deps` with no inputs; installs `pkg-config`, `libasound2-dev`, `libpipewire-0.3-dev`

- [ ] **Step 1: Create the composite action**

```yaml
# .github/actions/install-native-deps/action.yml
name: Install native audio deps
description: Install ALSA and PipeWire development packages for CPAL builds on Ubuntu
runs:
  using: composite
  steps:
    - name: Install pkg-config, ALSA, PipeWire
      shell: bash
      run: |
        sudo apt-get update
        sudo apt-get install -y pkg-config libasound2-dev libpipewire-0.3-dev
```

- [ ] **Step 2: Delete monolithic workflow**

```bash
rm .github/workflows/ci.yml
```

- [ ] **Step 3: Commit**

```bash
git add .github/actions/install-native-deps/action.yml
git rm .github/workflows/ci.yml
git commit -m "$(cat <<'EOF'
ci: add native-deps action and remove monolithic workflow

EOF
)"
```

---

### Task 2: Stable lint workflow

**Files:**
- Create: `.github/workflows/lint.yml`

**Interfaces:**
- Consumes: `./.github/actions/install-native-deps` before clippy
- Produces: workflow name `Lint` with jobs that block PRs when red

- [ ] **Step 1: Write `lint.yml`**

```yaml
name: Lint

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main, develop]

env:
  CARGO_TERM_COLOR: always

jobs:
  lint:
    name: fmt + clippy
    runs-on: ubuntu-latest
    defaults:
      run:
        working-directory: crates
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@master
        with:
          toolchain: stable
          components: rustfmt, clippy

      - name: Install native deps
        uses: ./.github/actions/install-native-deps

      - name: Cache cargo
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            crates/target
          key: ${{ runner.os }}-cargo-lint-${{ hashFiles('crates/Cargo.lock') }}
          restore-keys: |
            ${{ runner.os }}-cargo-lint-

      - name: Check formatting
        run: cargo fmt -- --check

      - name: Run clippy
        run: cargo clippy --all-targets --all-features -- -D warnings
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/lint.yml
git commit -m "$(cat <<'EOF'
ci: add stable lint workflow (fmt + clippy)

EOF
)"
```

---

### Task 3: Stable test workflow

**Files:**
- Create: `.github/workflows/test.yml`

**Interfaces:**
- Consumes: `./.github/actions/install-native-deps`
- Produces: workflow name `Test` running `cargo test --release --verbose` in `crates/`

- [ ] **Step 1: Write `test.yml`**

```yaml
name: Test

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main, develop]

env:
  CARGO_TERM_COLOR: always

jobs:
  test:
    name: cargo test
    runs-on: ubuntu-latest
    defaults:
      run:
        working-directory: crates
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@master
        with:
          toolchain: stable

      - name: Install native deps
        uses: ./.github/actions/install-native-deps

      - name: Cache cargo
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            crates/target
          key: ${{ runner.os }}-cargo-test-${{ hashFiles('crates/Cargo.lock') }}
          restore-keys: |
            ${{ runner.os }}-cargo-test-

      - name: Run tests
        run: cargo test --release --verbose
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/test.yml
git commit -m "$(cat <<'EOF'
ci: add stable test workflow

EOF
)"
```

---

### Task 4: Hybrid audit config + audit workflow

**Files:**
- Create: `crates/audit.toml`
- Create: `.github/workflows/audit.yml`
- Modify: `crates/Cargo.lock` (and manifests only if a direct dep bump is required)

**Interfaces:**
- Produces: `cargo audit` exits 0 when vulns are fixed or ignored with rationale; warnings do not fail

- [ ] **Step 1: Run audit locally to list current findings**

```bash
cargo install cargo-audit --locked
cd crates && cargo audit
```

Expected: vulnerabilities for `bytes` (RUSTSEC-2026-0007) and/or `crossbeam-epoch` (RUSTSEC-2026-0204), plus warning advisories.

- [ ] **Step 2: Bump vulnerable crates where possible**

```bash
cd crates
cargo update -p bytes --precise 1.11.1
cargo update -p crossbeam-epoch --precise 0.9.20
cargo audit
```

If `cargo update` cannot select those versions, record the blocker and add a temporary ignore with rationale in `audit.toml`.

- [ ] **Step 3: Write `crates/audit.toml`**

Start from a template; adjust IDs to match Step 2 output. Prefer empty `ignore` for vulns after successful bumps. Allow known warning advisories explicitly if `cargo audit` still fails on them (it should not — warnings are non-fatal by default; only add ignores if needed):

```toml
# Hybrid audit policy for #74:
# - Fail CI on unresolved vulnerabilities.
# - Prefer dependency bumps; ignore vulns only with rationale.
# - Warnings (unmaintained/unsound/yanked) are informational unless listed below.

[advisories]
ignore = [
    # Add RUSTSEC-… here only if a bump is not feasible, with a comment above each ID.
]
```

- [ ] **Step 4: Write `audit.yml`**

```yaml
name: Audit

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main, develop]

env:
  CARGO_TERM_COLOR: always

jobs:
  audit:
    name: cargo audit
    runs-on: ubuntu-latest
    defaults:
      run:
        working-directory: crates
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@master
        with:
          toolchain: stable

      - name: Install cargo-audit
        run: cargo install cargo-audit --locked

      - name: Run security audit
        run: cargo audit
```

- [ ] **Step 5: Verify audit is clean (or only documented ignores)**

```bash
cd crates && cargo audit
```

Expected: exit 0.

- [ ] **Step 6: Commit**

```bash
git add crates/audit.toml crates/Cargo.lock .github/workflows/audit.yml
git commit -m "$(cat <<'EOF'
ci: add hybrid cargo-audit workflow and audit.toml

EOF
)"
```

---

### Task 5: Daily beta warning-only workflow

**Files:**
- Create: `.github/workflows/rust-beta-audit.yml`

**Interfaces:**
- Consumes: `./.github/actions/install-native-deps`
- Produces: non-blocking daily beta lint+test on `main`

- [ ] **Step 1: Write `rust-beta-audit.yml`**

```yaml
name: Rust beta audit

on:
  schedule:
    # Daily 06:00 UTC
    - cron: '0 6 * * *'
  workflow_dispatch:

env:
  CARGO_TERM_COLOR: always

jobs:
  beta:
    name: beta fmt/clippy/test
    runs-on: ubuntu-latest
    continue-on-error: true
    defaults:
      run:
        working-directory: crates
    steps:
      - uses: actions/checkout@v4
        with:
          ref: main

      - name: Install Rust
        uses: dtolnay/rust-toolchain@master
        with:
          toolchain: beta
          components: rustfmt, clippy

      - name: Install native deps
        uses: ./.github/actions/install-native-deps

      - name: Cache cargo
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            crates/target
          key: ${{ runner.os }}-cargo-beta-${{ hashFiles('crates/Cargo.lock') }}
          restore-keys: |
            ${{ runner.os }}-cargo-beta-

      - name: Check formatting
        run: cargo fmt -- --check

      - name: Run clippy
        run: cargo clippy --all-targets --all-features -- -D warnings

      - name: Run tests
        run: cargo test --release --verbose
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/rust-beta-audit.yml
git commit -m "$(cat <<'EOF'
ci: add daily warning-only rust-beta-audit workflow

EOF
)"
```

---

### Task 6: Local sanity + PR verification notes

**Files:**
- Modify: none required (optional comment in PR body only)

- [ ] **Step 1: Local fmt/clippy smoke (if deps available)**

```bash
cd crates
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
```

Expected: pass (or fix formatting/clippy first in a separate commit).

- [ ] **Step 2: Confirm old workflow is gone**

```bash
test ! -f .github/workflows/ci.yml
ls .github/workflows/
```

Expected: `lint.yml`, `test.yml`, `audit.yml`, `rust-beta-audit.yml` only (plus any unrelated workflows).

- [ ] **Step 3: Open/push PR and confirm Actions**

Required green checks: `Lint`, `Test`, `Audit`.  
`Rust beta audit` should not run on the PR (schedule/dispatch only) and must not be a required status.

PR description must note: update branch protection to require the new check names and remove obsolete `CI` / `moon ci` / matrix jobs.

---

## Spec coverage checklist

| Spec requirement | Task |
|------------------|------|
| Split workflows | 2–5 |
| Native deps | 1 (+ used in 2, 3, 5) |
| Stable fmt/clippy | 2 |
| Stable test | 3 |
| Hybrid audit + audit.toml | 4 |
| No docs | (omitted by design) |
| Beta daily warning-only as `rust-beta-audit` | 5 |
| Delete monolithic ci.yml | 1 |
| cache@v4 | 2, 3, 5 |
| Acceptance / branch protection note | 6 |
