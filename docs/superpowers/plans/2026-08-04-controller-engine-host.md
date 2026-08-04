# ControllerEngine Host Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `ControllerEngine` (midir + app-data mappings) in `controller` and wire it through Tauri + Controllers settings / connect prompt.

**Architecture:** Single `controller` crate owns midir (git-pinned) and `ControllerEngine`. Tauri seeds `app_data/mappings`, pumps MIDI, implements `ActionPublish`, exposes invokes/events. FE Controllers settings + connect offer dialog.

**Tech Stack:** Rust midir (git), Tauri 2, React settings UI, existing `MappingSession` / buses

## Global Constraints

- midir git rev with `alsa >=0.9, <0.13` (e.g. `e5a60b5`) until crates.io catches up
- Copy-if-missing seed; update overwrites; ask every connect (no persist allow)
- Engine cmds via shared host path with `engine_publish`; library nav via `publish_evt`
- Ask-every-connect; [#133](https://github.com/geovannimp/rust-dj-engine/issues/133) out of scope

---

## File map

| File | Role |
|------|------|
| `crates/controller/Cargo.toml` | Add midir git dep |
| `crates/controller/src/engine.rs` | `ControllerEngine` + catalog/seed/update/attach |
| `crates/controller/src/lib.rs` | Export engine types |
| `crates/controller/tests/engine_seed.rs` | Seed/update filesystem tests |
| `apps/gui-app/src-tauri/Cargo.toml` | Depend on `controller` |
| `apps/gui-app/src-tauri/src/controller_host.rs` | Tauri manage, midir pump, ActionPublish, invokes |
| `apps/gui-app/src-tauri/src/lib.rs` | Setup + register commands |
| `apps/gui-app/src-tauri/src/bus_bridge.rs` | Extract shared cmd apply if needed |
| FE settings + types | Controllers panel, offer listener |

---

### Task 1: midir + ControllerEngine (filesystem + API)

**Files:** `crates/controller/Cargo.toml`, `src/engine.rs`, `src/lib.rs`, `tests/engine_seed.rs`

- [ ] Pin midir git; add `engine` module
- [ ] Implement seed/update/list_mappings without requiring open ports
- [ ] Test copy-if-missing and update overwrite
- [ ] Commit

### Task 2: Device poll + attach + MIDI pump

**Files:** `crates/controller/src/engine.rs`

- [ ] `list_devices` / `poll_devices` with midir; `MappingOffer` events
- [ ] `enable_mapping` / `disable_mapping` attach `MappingSession` + out port
- [ ] `pump` drains MIDI into session via `ActionPublish`
- [ ] Commit

### Task 3: Tauri host bridge

**Files:** `controller_host.rs`, `lib.rs`, `Cargo.toml`, optionally `bus_bridge.rs`

- [ ] Resolve shipped mappings path; seed on setup; background poll/pump
- [ ] Invokes + emit offers; ActionPublish to engine/library
- [ ] Commit

### Task 4: FE Controllers settings + connect prompt

**Files:** settings components, `SettingsPage`, types, toast/dialog

- [ ] Controllers section: list, enable/disable, update / update all
- [ ] Listen for offer → confirm → enable_mapping
- [ ] Commit

### Task 5: Verify

- [ ] `cargo test -p controller`; gui-app builds with cpal+controller
- [ ] Mark design acceptance checkboxes that are done
