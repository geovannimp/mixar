# moonrepo Orchestration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Adopt moon (`@moonrepo/cli`) so root scripts and CI run only affected Cargo + npm tasks (`gui-app` lint/format-check/build; rust format-check/lint/test/build).

**Architecture:** One moon workspace with two projects — `rust` (root Cargo workspace via `moon.yml`) and `gui-app` (`gui-app/moon.yml`). Package scripts remain the JS source of truth; moon shells `cargo` for Rust. Primary CI job is `moon ci` with full git history; beta/nightly stay a path-filtered matrix. Lefthook is unchanged.

**Tech Stack:** moon v2 (`@moonrepo/cli`), npm workspaces, Cargo workspace, GitHub Actions

## Global Constraints

- Orchestrator: moon via root `devDependency` `@moonrepo/cli` (not Turbo/Nx)
- Project IDs: `rust` (source `.`), `gui-app` (source `gui-app`)
- Shared task names: `lint`, `format-check`, `build`; rust also `test`
- `gui-app` package.json scripts stay source of truth; moon calls them
- No `website` / `docs` package stubs in this plan
- Do not put `gui-app/src-tauri` in rust moon inputs (excluded from Cargo workspace)
- Do not cache `target/` as a moon task output
- Lefthook staged-file hooks stay as-is
- Node 22 (`.nvmrc`); no proto requirement for v1
- Issue: #82
- Spec: `docs/superpowers/specs/2026-07-17-moonrepo-orchestration-design.md`

## File map

| File | Responsibility |
|------|----------------|
| `package.json` / `package-lock.json` | `@moonrepo/cli`; root `lint` / `format:check` / `build` → moon |
| `.moon/workspace.yml` | Project map + VCS (`main`) |
| `.moon/toolchains.yml` | Enable `rust` + `javascript`/`node` (align Node 22) |
| `moon.yml` | Project `rust`: cargo format-check / lint / test / build + inputs |
| `gui-app/moon.yml` | Project `gui-app`: npm script wrappers + inputs; `dev` not in CI |
| `.gitignore` | Ignore `.moon/cache` |
| `.github/workflows/ci.yml` | Primary `moon ci`; path-filtered rust matrix; keep security/docs |
| `README.md` / `AGENTS.md` | Install + moon commands + how to add npm packages |

---

### Task 1: Install moon + workspace/toolchain skeleton

**Files:**
- Create: `.moon/workspace.yml`
- Create: `.moon/toolchains.yml`
- Modify: `package.json`
- Modify: `package-lock.json` (via npm)
- Modify: `.gitignore`

**Interfaces:**
- Produces: `npx moon` binary available after `npm ci` / `npm install`
- Produces workspace projects map keys `rust` and `gui-app` (projects may warn until Task 2 adds `moon.yml` files — create stub `moon.yml` files in this task so `moon query projects` succeeds)

- [ ] **Step 1: Confirm branch**

```bash
git checkout feature/issue-82-moonrepo-orchestration
git status
```

Expected: on `feature/issue-82-moonrepo-orchestration` (create from `main` if missing).

- [ ] **Step 2: Install `@moonrepo/cli` at repo root**

```bash
npm install -D @moonrepo/cli
```

Expected: root `package.json` `devDependencies` includes `@moonrepo/cli`; lockfile updates.

- [ ] **Step 3: Create `.moon/workspace.yml`**

```yaml
# https://moonrepo.dev/docs/config/workspace
$schema: 'https://moonrepo.dev/schemas/workspace.json'

projects:
  rust: '.'
  gui-app: 'gui-app'

vcs:
  client: 'git'
  provider: 'github'
  defaultBranch: 'main'
```

- [ ] **Step 4: Create `.moon/toolchains.yml`**

```yaml
# https://moonrepo.dev/docs/config/toolchain
$schema: 'https://moonrepo.dev/schemas/toolchains.json'

rust: {}

javascript: {}

node:
  version: '22.0.0'
  packageManager: 'npm'

npm: {}
```

If moon rejects the exact Node version string, pin a concrete 22.x that matches local/CI (still major 22 per `.nvmrc`). Prefer enabling languages with minimal settings; do not require `.prototools` for v1.

- [ ] **Step 5: Add stub project configs so discovery works**

Create root `moon.yml`:

```yaml
$schema: 'https://moonrepo.dev/schemas/project.json'
language: 'rust'
layer: 'application'
id: 'rust'
workspace:
  inheritedTasks:
    include: []
```

Create `gui-app/moon.yml`:

```yaml
$schema: 'https://moonrepo.dev/schemas/project.json'
language: 'typescript'
layer: 'application'
id: 'gui-app'
workspace:
  inheritedTasks:
    include: []
```

(Task 2 fills in tasks.)

- [ ] **Step 6: Ignore moon cache**

Append to `.gitignore`:

```gitignore
# moon
.moon/cache
```

- [ ] **Step 7: Verify moon sees projects**

```bash
npx moon --version
npx moon query projects
```

Expected: version prints; output lists `rust` and `gui-app` (or equivalent project listing). If schema URLs 404, remove `$schema` lines or point at schemas generated under `.moon/cache` after first run.

- [ ] **Step 8: Commit**

```bash
git add package.json package-lock.json .moon/workspace.yml .moon/toolchains.yml moon.yml gui-app/moon.yml .gitignore
git commit -m "$(cat <<'EOF'
chore: add moon workspace skeleton for #82

Pin @moonrepo/cli and register rust + gui-app projects so local moon discovery works.
EOF
)"
```

---

### Task 2: Define rust + gui-app tasks and root scripts

**Files:**
- Modify: `moon.yml`
- Modify: `gui-app/moon.yml`
- Modify: `package.json`

**Interfaces:**
- Consumes: Task 1 workspace (`rust`, `gui-app`)
- Produces targets: `rust:format-check`, `rust:lint`, `rust:test`, `rust:build`, `gui-app:lint`, `gui-app:format-check`, `gui-app:build`
- Produces root npm scripts: `lint` → `moon run :lint`, `format:check` → `moon run :format-check`, `build` → `moon run :build`

- [ ] **Step 1: Replace root `moon.yml` with Cargo workspace tasks**

```yaml
$schema: 'https://moonrepo.dev/schemas/project.json'
language: 'rust'
layer: 'application'
id: 'rust'

workspace:
  inheritedTasks:
    include: []

env:
  CARGO_TERM_COLOR: 'always'

fileGroups:
  sources:
    - 'audio-core/**/*'
    - 'backend-null/**/*'
    - 'backend-miniaudio/**/*'
    - 'backend-cpal/**/*'
    - 'engine-core/**/*'
    - 'engine-dsp/**/*'
    - 'codec/**/*'
    - 'resampler/**/*'
    - 'library-core/**/*'
    - 'library/**/*'
    - 'library-adapters/**/*'
    - 'analyzer-core/**/*'
    - 'analyzer-stratum/**/*'
    - 'analyzer/**/*'
    - 'app-example/**/*'
    - 'tests/**/*'
    - 'Cargo.toml'
    - 'Cargo.lock'
  configs:
    - 'rustfmt.toml'
    - 'clippy.toml'
    - '.cargo/**/*'

tasks:
  format-check:
    command: 'cargo'
    args: ['fmt', '--', '--check']
    inputs:
      - '@group(sources)'
      - '@group(configs)'
    options:
      runInCI: true

  lint:
    command: 'cargo'
    args:
      - 'clippy'
      - '--all-targets'
      - '--all-features'
      - '--'
      - '-D'
      - 'warnings'
    inputs:
      - '@group(sources)'
      - '@group(configs)'
    options:
      runInCI: true

  test:
    command: 'cargo'
    args: ['test', '--release', '--verbose']
    inputs:
      - '@group(sources)'
      - '@group(configs)'
    options:
      runInCI: true

  build:
    command: 'cargo'
    args: ['build', '--release', '--verbose']
    inputs:
      - '@group(sources)'
      - '@group(configs)'
    options:
      runInCI: true
```

If `rustfmt.toml` / `clippy.toml` / `.cargo` do not exist, omit those config globs (do not create empty placeholder configs).

Do **not** list `target/` under `outputs`.

- [ ] **Step 2: Replace `gui-app/moon.yml` with npm script tasks**

```yaml
$schema: 'https://moonrepo.dev/schemas/project.json'
language: 'typescript'
layer: 'application'
id: 'gui-app'

workspace:
  inheritedTasks:
    include: []

fileGroups:
  sources:
    - 'src/**/*'
    - 'index.html'
    - 'vite.config.*'
    - 'tsconfig*.json'
    - 'package.json'
    - '.oxlintrc.json'
    - '.oxfmtrc.json'
    - 'components.json'

tasks:
  lint:
    command: 'npm'
    args: ['run', 'lint']
    inputs:
      - '@group(sources)'
    options:
      runInCI: true

  format-check:
    command: 'npm'
    args: ['run', 'format:check']
    inputs:
      - '@group(sources)'
    options:
      runInCI: true

  build:
    command: 'npm'
    args: ['run', 'build']
    inputs:
      - '@group(sources)'
    options:
      runInCI: true

  dev:
    command: 'npm'
    args: ['run', 'dev']
    local: true
    options:
      runInCI: false
      persistent: true
```

Adjust source globs if gui-app layout differs (keep `src-tauri` out of these JS task inputs unless a future task intentionally adds Tauri cargo checks).

- [ ] **Step 3: Wire root `package.json` scripts**

Update `scripts` to:

```json
"scripts": {
  "prepare": "lefthook install",
  "lint": "moon run :lint",
  "format:check": "moon run :format-check",
  "build": "moon run :build",
  "dev:gui": "npm run dev -w gui-app",
  "build:gui": "moon run gui-app:build",
  "tauri": "npm run tauri -w gui-app"
}
```

Keep `prepare` / `dev:gui` / `tauri` outside the CI task graph.

- [ ] **Step 4: Run gui-app tasks**

```bash
npx moon run gui-app:format-check
npx moon run gui-app:lint
npx moon run gui-app:build
```

Expected: all exit 0 on a clean tree (build may take longer; needs gui-app deps already installed via root `npm install`).

- [ ] **Step 5: Run rust tasks (may be slow)**

```bash
npx moon run rust:format-check
npx moon run rust:lint
```

Expected: exit 0 (same gates as today’s `cargo fmt --check` / clippy). Optionally run `npx moon run rust:test` once if time allows; otherwise CI will cover it.

- [ ] **Step 6: Run root aggregate scripts**

```bash
npm run format:check
npm run lint
```

Expected: moon runs `:format-check` and `:lint` across projects that define them; exits 0.

- [ ] **Step 7: Commit**

```bash
git add moon.yml gui-app/moon.yml package.json
git commit -m "$(cat <<'EOF'
feat: define moon tasks for rust and gui-app

Expose format-check, lint, build (and rust test) via moon, with root npm scripts delegating to the task graph.
EOF
)"
```

---

### Task 3: Rewire GitHub Actions to `moon ci`

**Files:**
- Modify: `.github/workflows/ci.yml`

**Interfaces:**
- Consumes: Task 2 targets with `runInCI: true`
- Produces: primary job `ci` running `npx moon ci`; secondary `rust-matrix` for beta/nightly on Rust paths only; existing `security` / `docs` jobs retained

- [ ] **Step 1: Replace stable test/build/lint jobs with a primary `ci` job**

Rewrite `.github/workflows/ci.yml` to this structure (keep `on:` / `env:` headers; preserve `security` and `docs` jobs largely as today):

```yaml
name: CI

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main, develop]

env:
  CARGO_TERM_COLOR: always
  # Prefer PATH toolchains installed by Actions over moon downloading via proto
  MOON_TOOLCHAIN_FORCE_GLOBALS: 'true'

jobs:
  ci:
    name: moon ci
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
          filter: blob:none

      - name: Install Rust
        uses: dtolnay/rust-toolchain@master
        with:
          toolchain: stable
          components: rustfmt, clippy

      - name: Cache cargo
        uses: actions/cache@v3
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-moon-${{ hashFiles('**/Cargo.lock') }}
          restore-keys: |
            ${{ runner.os }}-cargo-moon-

      - name: Setup Node
        uses: actions/setup-node@v4
        with:
          node-version-file: .nvmrc
          cache: npm

      - name: Install npm dependencies
        run: npm ci

      - name: Run moon ci (affected)
        run: npx moon ci

  rust-matrix:
    name: Rust matrix (${{ matrix.rust }})
    runs-on: ubuntu-latest
    strategy:
      matrix:
        rust: [beta, nightly]
    steps:
      - uses: actions/checkout@v4

      - name: Detect Rust changes
        id: filter
        uses: dorny/paths-filter@v3
        with:
          filters: |
            rust:
              - 'audio-core/**'
              - 'backend-*/**'
              - 'engine-*/**'
              - 'codec/**'
              - 'resampler/**'
              - 'library*/**'
              - 'analyzer*/**'
              - 'app-example/**'
              - 'tests/**'
              - 'Cargo.toml'
              - 'Cargo.lock'
              - '**/Cargo.toml'

      - name: Install Rust
        if: steps.filter.outputs.rust == 'true'
        uses: dtolnay/rust-toolchain@master
        with:
          toolchain: ${{ matrix.rust }}
          components: rustfmt, clippy

      - name: Cache cargo
        if: steps.filter.outputs.rust == 'true'
        uses: actions/cache@v3
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-${{ matrix.rust }}-${{ hashFiles('**/Cargo.lock') }}
          restore-keys: |
            ${{ runner.os }}-cargo-${{ matrix.rust }}-

      - name: Check formatting
        if: steps.filter.outputs.rust == 'true'
        run: cargo fmt -- --check

      - name: Run clippy
        if: steps.filter.outputs.rust == 'true'
        run: cargo clippy --all-targets --all-features -- -D warnings

      - name: Run tests
        if: steps.filter.outputs.rust == 'true'
        run: cargo test --release --verbose

  security:
    name: Security Audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@master
        with:
          toolchain: stable

      - name: Install cargo-audit
        run: cargo install cargo-audit

      - name: Run security audit
        run: cargo audit

  docs:
    name: Documentation
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@master
        with:
          toolchain: stable

      - name: Cache cargo registry
        uses: actions/cache@v3
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-docs-${{ hashFiles('**/Cargo.lock') }}
          restore-keys: |
            ${{ runner.os }}-cargo-docs-

      - name: Build documentation
        run: cargo doc --no-deps --document-private-items

      - name: Deploy to GitHub Pages
        uses: peaceiris/actions-gh-pages@v3
        if: github.ref == 'refs/heads/main'
        with:
          github_token: ${{ secrets.GITHUB_TOKEN }}
          publish_dir: ./target/doc
```

Notes for the implementer:

- Remove the old standalone `test` (stable+beta+nightly), `build`, and `lint` jobs — their stable coverage moves into `moon ci`; beta/nightly stay in `rust-matrix`.
- `dorny/paths-filter@v3` skips matrix work on frontend-only PRs. On `push` events without a PR base, if the filter always reports false incorrectly, set `filters` with `base: ${{ github.event.repository.default_branch }}` or fall back to running the matrix unconditionally on `push` only — prefer correct skip on PRs.
- Do **not** leave `npm run format:check -w gui-app` / `npm run lint -w gui-app` in the workflow; moon owns those.
- Optional follow-up (not required): `moonrepo/run-report-action@v1` after `moon ci`.

- [ ] **Step 2: Local dry-run of affected detection**

```bash
npx moon ci --base main --head HEAD
```

Expected: moon prints an action graph and runs only tasks affected vs `main` (may run many tasks if the branch differs a lot). Command must not error on “shallow clone” locally.

To sanity-check frontend-only behavior without waiting for full rust test, temporarily note which targets moon schedules when only `gui-app/src` differs (use a throwaway commit or `moon query touched-files` / moon’s affected reporting if available). At minimum, confirm `moon ci` no longer depends on `-w gui-app` scripts in YAML.

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "$(cat <<'EOF'
ci: run affected checks through moon ci

Replace hand-wired workspace npm lint steps and stable cargo jobs with moon ci; keep beta/nightly behind a Rust path filter.
EOF
)"
```

---

### Task 4: Documentation (README + AGENTS.md)

**Files:**
- Modify: `README.md`
- Modify: `AGENTS.md`

**Interfaces:**
- Consumes: Task 2 scripts + Task 3 CI shape
- Produces: documented install, moon commands, and “add npm package” checklist from the spec

- [ ] **Step 1: Update README architecture tree**

In the layout block near the top of `README.md`, add moon entries, e.g.:

```text
├─ package.json        # npm workspaces root (gui-app + lefthook + @moonrepo/cli)
├─ .moon/              # moon workspace + toolchains
├─ moon.yml            # rust (Cargo workspace) moon project
├─ lefthook.yml        # pre-commit rustfmt + oxfmt/oxlint (staged files)
```

And under `gui-app/`, note `moon.yml` exists alongside the Tauri app.

- [ ] **Step 2: Update README Development / Git hooks section**

Replace the GUI snippet and CI blurb so they mention moon. Concrete text to merge in:

```markdown
### Git hooks

Run `npm install` once at the **repo root**. That installs [lefthook](https://lefthook.dev) and [moon](https://moonrepo.dev) (`@moonrepo/cli`), wires pre-commit hooks, and enables the task graph.

Pre-commit still runs `rustfmt` / `oxfmt` / `oxlint --fix` on **staged** files only (`stage_fixed`). CI uses `moon ci` for affected full-package checks.

- Skip a lefthook job: `LEFTHOOK_EXCLUDE=cargo-fmt` / `oxfmt` / `oxlint`
- Disable lefthook: `LEFTHOOK=0 git commit ...`
- Emergency only: `git commit --no-verify`

### moon task runner

```bash
npm install                 # root — hooks + gui-app + moon
npm run lint                # moon run :lint
npm run format:check        # moon run :format-check
npm run build               # moon run :build
npx moon run gui-app:dev    # or: npm run dev:gui
npx moon ci --base main     # locally mimic affected CI
```

#### Adding a new npm workspace package (e.g. `website`, `docs`)

1. Add the directory to root `package.json` `workspaces`.
2. Add `package.json` with `lint`, `format:check`, and `build` scripts.
3. Add `moon.yml` (`language: typescript`) whose tasks call those scripts; set `runInCI: false` on `dev`.
4. Register the project in `.moon/workspace.yml` if not covered by a glob.
5. `npm install` at root; verify `npx moon run <id>:lint` and that `npx moon ci --base main` only runs it when that package changes.
```

Update the **CI/CD** subsection to say the primary gate is `moon ci` (affected), with a Rust beta/nightly matrix on Rust path changes, plus security audit and docs jobs.

- [ ] **Step 3: Update `AGENTS.md` Learned Workspace Facts**

Change the npm/lefthook bullet to also mention moon, e.g.:

```markdown
- Root `npm install` installs [lefthook](https://lefthook.dev) and [moon](https://moonrepo.dev) (`@moonrepo/cli`) via the npm workspace root; `prepare` → `lefthook install`. Pre-commit runs `rustfmt` on staged `*.rs` and `oxfmt`/`oxlint --fix` on staged `*.{ts,tsx}` with `stage_fixed`. Skip jobs: `LEFTHOOK_EXCLUDE=cargo-fmt` / `oxfmt` / `oxlint`; disable: `LEFTHOOK=0`; emergency: `git commit --no-verify`. Prefer not bypassing the hook. Frontend tooling expects Node 22 (`.nvmrc`). Affected full-package checks use `npx moon ci` / root scripts `lint`, `format:check`, `build`.
```

- [ ] **Step 4: Commit**

```bash
git add README.md AGENTS.md
git commit -m "$(cat <<'EOF'
docs: document moon install and affected task commands

Explain root moon scripts, CI moon ci usage, and how to add future npm workspace packages.
EOF
)"
```

---

## Spec coverage checklist

| Spec requirement | Task |
|------------------|------|
| `@moonrepo/cli` at root | 1 |
| `.moon/workspace.yml` + toolchains | 1 |
| Project `rust` + `gui-app` | 1–2 |
| Tasks lint / format-check / build (+ rust test) | 2 |
| Root scripts delegate to moon | 2 |
| Package scripts remain source of truth | 2 |
| `moon ci` primary CI + full history | 3 |
| Frontend-only skips rust in primary pipeline | 3 (affected inputs) |
| Beta/nightly secondary path filter | 3 |
| Lefthook unchanged | (no task touches `lefthook.yml`) |
| Docs + add-package checklist | 4 |
| No website/docs stubs | (omitted) |
| No `target/` moon outputs | 2 |
| No src-tauri in rust inputs | 2 |

## Placeholder / consistency self-review

- Task IDs and names match the design (`rust`, `gui-app`, `format-check`, `lint`, `build`, `test`).
- Root npm `format:check` maps to moon task id `format-check` (hyphen), not `format_check`.
- No TBD/TODO left in steps; CI YAML is fully specified.
- `MOON_TOOLCHAIN_FORCE_GLOBALS` documented so CI uses Actions-installed Rust/Node.
