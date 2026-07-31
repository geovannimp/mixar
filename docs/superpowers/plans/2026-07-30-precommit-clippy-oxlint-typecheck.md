# Pre-commit clippy + oxlint typeCheck Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire affected-crate clippy and oxlint typeCheck into pre-commit `lint:staged` so #99 class of CI failures fail locally.

**Architecture:** Moon `lint-files` for `crates` maps staged `.rs` → `cargo clippy -p …`. Gui-app enables oxlint `typeAware`/`typeCheck` via `oxlint-tsgolint`. Lefthook lint glob widened so Rust tasks run.

**Tech Stack:** moon, lefthook, cargo clippy, oxlint + oxlint-tsgolint.

**Spec:** `docs/superpowers/specs/2026-07-30-precommit-clippy-oxlint-typecheck-design.md`

## Global Constraints

- Clippy when any crates `.rs` is staged (moon project gate; same flags as CI `lint`).
- Oxlint typeCheck (not separate `tsc --noEmit`) for the TS gate.
- Do not add src-tauri clippy in this slice.
- Prefer fewest files; reuse moon `affectedFiles` skip patterns from `format-files`.

## File map

| File | Role |
|------|------|
| `crates/moon.yml` | Add `lint-files` → workspace clippy when `.rs` staged |
| `apps/gui-app/package.json` | Add `oxlint-tsgolint` |
| `apps/gui-app/oxlint.config.ts` | `typeAware` + `typeCheck` |
| `lefthook.yml` | Drop TS-only lint glob |
| `package-lock.json` | Lockfile for new dep |
| Spec | Status accepted when done |

---

### Task 1: Rust lint-files

**Files:**
- Modify: `crates/moon.yml`

- [ ] **Step 1: Add lint-files**

Same clippy args as CI `lint`. `affectedFiles.pass: false` + `.rs` filter + `passDotWhenNoResults: false` so the task runs only when `.rs` is staged and does not append file paths to cargo.

- [ ] **Step 2: Smoke** `npx moon run rust:lint-files` → clippy workspace.

- [ ] **Step 3: Commit**

```
chore(crates): add lint-files clippy when Rust sources staged
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
