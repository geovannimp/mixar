# gui-app components domain layout — Implementation Plan

> **For agentic workers:** Execute task-by-task. Steps use checkbox syntax.

**Goal:** Move `apps/gui-app/src/components` into domain folders per `docs/superpowers/specs/2026-08-02-gui-components-domain-layout-design.md` with import updates only.

**Architecture:** `git mv` into `deck/`, `deck/pads/`, `mixer/`, `library/`, `waveform/`, `shell/`, `dnd/`, `dialogs/`; rewrite imports to concrete `@/components/<domain>/…` paths; update `gui-app.mdc`.

**Tech Stack:** React/TS gui-app, git mv, oxlint/tsc/vitest.

## Global Constraints

- No barrels / no directory imports.
- No behavior or UI changes.
- Keep `ui/` and `settings/` in place.

---

### Task 1: Move files

- [x] Create domain dirs; `git mv` membership from the spec (rename `deck-pads` → `deck/pads`).
- [x] Delete unused `WaveformWindowMarkers.tsx` if still present and unreferenced.

### Task 2: Fix imports

- [x] Rewrite `@/components/<OldName>` and `@/components/deck-pads/…` across `apps/gui-app`.
- [x] Fix broken relative `./` imports after moves (prefer `@/` for cross-folder).

### Task 3: Cursor rule + verify

- [x] Update `.cursor/rules/gui-app.mdc` domain/`deck/pads` notes.
- [x] `tsc --noEmit`, `oxlint`, `npm test` in gui-app.
- [ ] Mark spec status implemented; commit; push; open PR.
