# Flutter EngineTransport + EngineBuses — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Attach host-owned omnibus buses to `Engine`, deprecate `EngineSession` to a shim, and add Flutter FRB `EngineTransport` + `AudioBackendTransport` (host API only).

**Architecture:** Host creates `EngineBuses`, injects clones into `Engine`, owns `spawn_engine_worker` JoinHandle. Flutter `EngineTransport` is that host. Tauri keeps compiling via thin `EngineSession` shim. Device listing is a separate `AudioBackendTransport`.

**Tech Stack:** Rust (`engine-core`, `engine-api`, `host-flutter`), flutter_rust_bridge 2.12, existing control thread.

## Global Constraints

- Worker `JoinHandle` must not live inside `Mutex<Engine>`
- `publish_evt` for controller must work via `EngineBuses` without locking the engine mutex
- Flutter UI stays placeholder (no mixer/settings wiring)
- Thin bus: Play/Pause cmds; host-side load; Status/Updated/Position/Levels/Error/Notice evts
- Load prepare happens outside the engine lock
- `EngineTransport::start` takes library + config (not a bare backend name)
- Prefer shortest diffs; keep `Arc<Mutex<Option<Engine>>>` in the cmd worker

## File map

| Path | Role |
|------|------|
| `crates/engine-core/src/bus.rs` | Add `EngineBuses` + publish helpers |
| `crates/engine-core/src/engine.rs` | `set_buses` / publish / subscribe / `is_running` |
| `crates/engine-core/src/control.rs` | Export `spawn_engine_worker` / `EngineWorker` |
| `crates/engine-core/src/session.rs` | Thin deprecated shim |
| `crates/engine-core/src/lib.rs` | Re-exports |
| `crates/engine-core/tests/engine_buses.rs` | Engine + buses smoke (no session) |
| `crates/host-flutter/src/api/engine.rs` | Both FRB transports + DTOs + stream |
| `crates/host-flutter/src/api/library.rs` | `#[frb(ignore)]` library_arc / cmd_bus |
| `crates/host-flutter/tests/smoke_null_backend.rs` | Rewrite onto new types |
| `crates/host-flutter/tests/engine_transport.rs` | Play/load/stream smoke |
| FRB generated files | Regenerate after Rust API change |

---

### Task 1: `EngineBuses` + engine attach + public worker spawn

**Files:** engine-core as in the file map.

- [x] Implement `EngineBuses`, `Engine::set_buses`, `spawn_engine_worker`, `EngineSession` shim, `engine_buses.rs` tests.
- [x] `cargo test --manifest-path crates/Cargo.toml -p engine-core`

### Task 2: Flutter `AudioBackendTransport` + `EngineTransport`

- [x] `LibraryTransport` ignore accessors; replace free engine fns with the two opaque types; host-side load; evt stream with coalescing.
- [x] `cargo test --manifest-path crates/Cargo.toml -p host_flutter`

### Task 3: FRB regenerate

- [x] `cd apps/gui-flutter && mise exec -- flutter_rust_bridge_codegen generate`
