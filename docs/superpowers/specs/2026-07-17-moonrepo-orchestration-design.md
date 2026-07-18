# moonrepo Monorepo Orchestration Design

**Issue:** [#82](https://github.com/geovannimp/rust-dj-engine/issues/82)  
**Date:** 2026-07-17

## Goal

Adopt [moon](https://moonrepo.dev) as the monorepo task orchestrator so root scripts and CI run **only affected** work across **Cargo + npm**. Frontend-only changes must not run Rust tests; Rust-only changes must not run frontend lint/build. Package-local scripts stay the source of truth; lefthook staged-file hooks stay as-is.

## Requirements

| Decision | Choice |
|----------|--------|
| Orchestrator | [moon](https://moonrepo.dev) v2 via root `devDependency` `@moonrepo/cli` |
| Why moon (not Turbo/Nx) | Native multi-language support including [Rust](https://moonrepo.dev/docs/guides/rust/handbook); one affected graph for Cargo + npm |
| Install | `npm install -D @moonrepo/cli` at repo root (same DX pattern as turbo); invoke via `npx moon` / root scripts |
| JS packages in v1 | Existing `gui-app` only; document how to add `website` / `docs` later (no stubs in this issue) |
| Rust modeling | **One** moon project at the Cargo workspace root (not per-crate); moon shells `cargo` — does not replace Cargo |
| Root task surface | Shared task names where possible: `lint`, `format-check`, `build`; Rust also `test` (clippy lives under rust `lint`) |
| Moon project IDs | `rust` — root `moon.yml` (Cargo workspace); `gui-app` — `gui-app/moon.yml` |
| CI | Primary quality gate is `moon ci` (affected + `runInCI`); full git history (`fetch-depth: 0`) |
| Frontend CI tasks | `lint`, `format-check`, **and** `build` (tsc + vite) when `gui-app` is affected |
| Lefthook | Unchanged; staged-file rustfmt / oxfmt / oxlint remain outside moon |
| Remote cache | Out of scope for v1; optional local `.moon/cache` persistence later |
| `proto` / full toolchain takeover | Optional; prefer pinning Node via existing `.nvmrc` + `actions/setup-node`; Rust via existing `dtolnay/rust-toolchain` or moon rust enablement without forcing every machine onto proto in v1 |
| Cargo as source of truth | Yes — moon does not move crates into a JS graph or invent parallel Rust build systems |

### Non-goals (this issue)

- Scaffolding `website` / `docs` packages
- Replacing lefthook
- Remote / distributed moon cache
- Per-crate moon projects for every Cargo member
- Forcing Tauri (`gui-app/src-tauri`, excluded from Cargo workspace) into the root Cargo moon tasks

## Architecture

```text
rust-dj-engine/
├─ package.json                 # workspaces + @moonrepo/cli; scripts → moon
├─ .moon/
│  ├─ workspace.yml             # project globs, VCS default branch
│  └─ toolchains.yml            # enable rust + node (versions aligned with repo)
├─ moon.yml                     # project id `rust` (Cargo workspace; language: rust)
├─ gui-app/
│  ├─ package.json              # lint / format:check / build (source of truth)
│  └─ moon.yml                  # language: typescript; tasks → npm scripts
├─ lefthook.yml                 # unchanged
└─ .github/workflows/ci.yml     # moon ci as primary affected pipeline
```

**Boundaries**

- **moon** — task graph, inputs hashing, affected detection, `moon ci`.
- **Cargo** — crate graph, builds, tests; moon runs workspace-level `cargo` commands.
- **npm workspaces** — package discovery and scripts; moon does not duplicate oxlint/oxfmt/vite command lines.
- **lefthook** — fast pre-commit on staged files only.

### Affected behavior (motivation)

| Change set | Expected moon CI work |
|------------|------------------------|
| Only `gui-app/**` (TS/frontend) | `gui-app` lint / format-check / build |
| Only Cargo members / `Cargo.*` | Root rust format / lint (clippy) / test / build as configured |
| Both / shared root config that affects both | Both project graphs’ affected tasks |
| Docs-only markdown under `docs/` (no moon inputs) | No lint/test/build tasks unless those paths are listed as inputs |

Exact input globs must cover this repo’s layout (top-level crates, not `crates/*`).

## Components

### Root npm

- Add `@moonrepo/cli` as a root `devDependency`.
- Replace ad-hoc `-w gui-app` checks with moon-backed scripts:
  - `lint` → `moon run :lint`
  - `format:check` → `moon run :format-check`
  - `build` → `moon run :build`
- Keep convenience scripts that are not CI graph tasks: `dev:gui`, `tauri`, `prepare` (lefthook). Mark long-running/dev tasks `runInCI: false` in moon config.

### `.moon/workspace.yml`

- Register projects: `rust` (root) + `gui-app` (and later `website`, `docs` via globs such as `*` / explicit paths).
- Set `vcs.defaultBranch` to `main` (CI also targets `develop`; `moon ci` uses provider-detected base/head for PRs).

### `.moon/toolchains.yml`

- Enable `rust` and `node` so moon language plugins are active.
- Align Node with `.nvmrc` (22).
- Do not require proto for v1 success; CI may install Node/Rust with existing Actions and set `MOON_TOOLCHAIN_FORCE_GLOBALS` if needed so moon uses PATH toolchains.

### Root `moon.yml` (Cargo workspace)

Single project wrapping the Cargo workspace (moon handbook “workspaces” pattern):

| Task | Command | `runInCI` |
|------|---------|-----------|
| `format-check` | `cargo fmt -- --check` | affected |
| `lint` | `cargo clippy --all-targets --all-features -- -D warnings` | affected |
| `test` | `cargo test --release` (include `backend-null` coverage equivalent to today’s CI) | affected |
| `build` | `cargo build --release` (add debug build only if still required after consolidation) | affected |

**Inputs / file groups** must match this repo: member crate paths (`audio-core/**`, `engine-core/**`, …), root `Cargo.toml` / `Cargo.lock`, and shared test paths — **not** `gui-app/src-tauri` (excluded from the Cargo workspace).

Do **not** declare the entire `target/` directory as a moon task output (moon handbook: incompatible with moon’s tarball cache). Continue using Actions `actions/cache` (or `moonrepo/setup-rust`) for `target/` / registry as today.

### `gui-app/moon.yml`

| Task | Implementation | Notes |
|------|----------------|-------|
| `lint` | `npm run lint` (package script → oxlint) | CI when affected |
| `format-check` | `npm run format:check` | CI when affected |
| `build` | `npm run build` (tsc + vite) | CI when affected |
| `dev` / `tauri` | optional wrappers | `runInCI: false` |

Inputs: `gui-app` sources, configs, `package.json`; exclude `dist` / `node_modules`.

### CI (`.github/workflows/ci.yml`)

**Primary job:** `moon ci`

1. `actions/checkout` with `fetch-depth: 0` and `filter: blob:none` (required for accurate affected detection).
2. Setup Node from `.nvmrc` + `npm ci` (installs `@moonrepo/cli` + workspace deps).
3. Setup Rust stable (+ `rustfmt`, `clippy`) compatible with current lint/test needs.
4. `npx moon ci` (or `npm exec moon -- ci`).
5. Optional: `moonrepo/run-report-action` for PR summaries.

**What this replaces:** the current combined `lint` job’s hand-wired `npm run … -w gui-app` steps, and folds stable-path Rust format/clippy/test/build into the same affected pipeline where practical.

**Preserved outside or beside moon (v1):**

- **Toolchain matrix** (`stable` / `beta` / `nightly`): moon pins one effective Rust toolchain for the primary job. Keep a **secondary** matrix job for beta/nightly that runs only when Rust workspace paths change (path filters or explicit `moon ci` target subset). Do not block v1 on teaching moon multi-toolchain matrices.
- **`cargo audit` / docs deploy:** remain separate jobs (security/docs); not required to be moon tasks in v1 unless trivial to add with `runInCI` carefully set.

**Dev tasks:** anything that starts Vite/Tauri must have `runInCI: false`.

### Lefthook

No change required for acceptance. Pre-commit stays staged-file oxfmt/oxlint/rustfmt. Moon is for full-package / CI orchestration, not commit-time file lists.

### Docs

Update README (+ `AGENTS.md` as needed):

- `npm install` installs lefthook **and** moon.
- Common commands: `npx moon run gui-app:lint`, root `npm run lint` / `format:check` / `build`, `npx moon ci` locally against a base branch.
- **How to add a new npm workspace package** (checklist below).

## Adding a new npm package (e.g. `website`, `docs`)

1. Add the directory to root `package.json` `workspaces`.
2. Add package `package.json` with `lint`, `format:check`, and `build` scripts (same names as `gui-app` where applicable).
3. Add `moon.yml` with `language: typescript` (or javascript) and tasks that call those scripts; set inputs; mark `dev` as `runInCI: false`.
4. Register the project in `.moon/workspace.yml` if not covered by an existing glob.
5. Run `npm install` at root; verify `npx moon run <id>:lint` and that `npx moon ci --base main` only runs the new package when it alone changes.

## Error handling

- Missing `node_modules` / moon binary: fail fast; CI always `npm ci` first.
- Shallow clone in CI: forbidden for the moon job (`fetch-depth: 0`); document why.
- Task failure: `moon ci` fails the job (same gate as today’s lint/test).
- Unaffected projects: tasks skipped — not treated as failure.
- Lefthook failures remain independent of moon.

## Testing / verification

1. `npm install` at root installs `@moonrepo/cli`.
2. `npx moon run gui-app:lint` / `format-check` / `build` succeed.
3. `npx moon run` root rust `format` / `lint` / `test` (or equivalent target IDs) succeed on a clean tree.
4. Simulate frontend-only diff: moon reports/runs only `gui-app` CI tasks (not cargo test).
5. Simulate rust-only diff: moon skips `gui-app` build/lint.
6. CI workflow uses `moon ci` with full history; frontend no longer invoked only via `npm run … -w gui-app`.
7. Lefthook still formats staged TS/Rust as before.

## Acceptance

1. Orchestrator config checked in (`.moon/*`, root + `gui-app` `moon.yml`); docs explain adding a new npm workspace package.
2. Root scripts invoke moon for at least frontend `lint`, `format:check`, and `build`.
3. CI uses `moon ci` for affected frontend + rust quality tasks (stable primary path).
4. `gui-app` package scripts remain the source of truth; moon calls them.
5. Frontend-only changes do not run Rust test/clippy in the primary affected pipeline.
6. Lefthook behavior preserved.
7. README / `AGENTS.md` updated for install and common moon commands.

## Related

- [#82](https://github.com/geovannimp/rust-dj-engine/issues/82) — this issue
- [#76](https://github.com/geovannimp/rust-dj-engine/issues/76) — frontend lint/format (deferred orchestrator)
- [#73](https://github.com/geovannimp/rust-dj-engine/issues/73) — lefthook / npm workspace bootstrap
- Spec: `docs/superpowers/specs/2026-07-17-gui-app-lint-design.md`
