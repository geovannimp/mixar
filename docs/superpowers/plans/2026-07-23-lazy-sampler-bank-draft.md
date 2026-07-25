# Lazy sampler bank draft (no ensure_default)

> **For agentic workers:** implement top-to-bottom; commit after each task if asked.

**Goal:** Remove `ensure_default_sampler_bank`. When no banks exist, start an unsaved draft the same way as UI `create_sampler_bank`; persist only on first edit.

**Architecture:** App-layer draft only (`draft_sampler_bank`). Library never auto-creates banks.

## Task 1: Remove library ensure_default

- Delete `ensure_default_bank` + `LibraryManager::ensure_default_sampler_bank`
- Drop re-exports; fix tests to use `create_bank`

## Task 2: Shared draft helper in deck_sampler

- Extract draft creation from `create_sampler_bank` into `start_draft_sampler_bank`
- `ensure_active_bank`: if no persisted banks and no draft, start draft; wire `ensure_sampler_ready`, `resolve_bank_id` (`&mut`), delete fallbacks, `select_bank_for_track_load`

## Task 3: Spec + verify

- Update sampler-banks design: seed = draft-on-need, not DB seed
- `cargo check` / sampler tests
