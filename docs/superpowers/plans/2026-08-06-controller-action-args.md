# Controller Action Named Args Implementation Plan

> **For agentic workers:** Inline execution. Checkbox steps for tracking.

**Goal:** Named action args in map.toml; hard-break old `_N` / mode-suffix leaves.

**Architecture:** Extend `parse_action_id` to parse `(key:value,…)` into args; catalog validates base leaf + schema; resolve uses args.

**Tech Stack:** Rust `controller` crate (`action_id`, `catalog`, `action`), shipped maps/fixtures.

## Global Constraints

- Named args only; 1-based deck/n/slot; no upper caps
- Integer beats only ([#137](https://github.com/geovannimp/rust-dj-engine/issues/137) for f32)
- `beat_jump(beats:±N)` replaces fwd/back leaves
- `load_to_deck(deck:N)` (not `load_focused_to_deck`)

---

### Task 1: Parse + catalog schemas + resolve + migrate

**Files:** `action_id.rs`, `catalog.rs`, `action.rs`, maps/fixtures, specs, tests

- [x] Parser returns leaf + args map
- [x] Leaf arg schemas; `is_known_action` validates
- [x] Resolve uses args; drop `_N` match arms
- [x] Migrate DDJ + fixtures; update docs; tests; commit; resolve PR thread
