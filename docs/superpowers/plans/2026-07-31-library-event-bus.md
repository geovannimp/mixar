# Library Event Bus Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `library-api` + `LibrarySession` omnibus cmd/evt, Tauri bytes bridge, and FE publish/subscribe; migrate track analysis onto the bus.

**Architecture:** Mirror engine: `library-api` owns MessagePack wire types; `LibrarySession` in `library` owns buses + worker + `LibraryManager`; Tauri `library_publish` / `library://bus`; FE `LibraryTransport` publish/subscribe with fire-and-forget analyze.

**Tech Stack:** Rust workspace, omnibus 0.1, rmp-serde, Tauri 2, React, MessagePack TS (`@msgpack/msgpack` + zod).

## Global Constraints

- MessagePack named maps end-to-end (same as engine).
- Host bridges bytes only; no Tauri-owned library event mirror.
- Analyze is fire-and-forget on FE; results arrive on evt.
- Do not migrate other library invokes in this PR.
- Spec: `docs/superpowers/specs/2026-07-31-library-event-bus-design.md`.

---

### Task 1: `library-api` crate

**Files:**
- Create: `crates/library-api/Cargo.toml`
- Create: `crates/library-api/src/{lib,origin,kind,payload,wire}.rs`
- Create: `crates/library-api/tests/msgpack_roundtrip.rs`
- Modify: `crates/Cargo.toml` (workspace members)

**Produces:** `Origin::{Library, Track(String)}`, `Kind::{AnalyzeTrack, TrackAnalyzed, Error, Notice}`, `CmdBody::AnalyzeTrack { track_id, force }` / `Empty`, `EvtBody::{TrackAnalyzed, Error, Notice, Empty}`, `TrackSummary`, encode/decode helpers, `WireMessage`.

- [ ] **Step 1:** Add crate + wire types + roundtrip test; run `cargo test -p library-api --manifest-path crates/Cargo.toml`
- [ ] **Step 2:** Commit

### Task 2: `LibrarySession` + worker in `library`

**Files:**
- Create: `crates/library/src/{bus,session,worker}.rs`
- Modify: `crates/library/src/lib.rs`, `crates/library/Cargo.toml`
- Create: `crates/library/tests/session_analyze_bus.rs`

**Produces:** `LibrarySession::open` / `open_in_memory`, `library()`, `set_analysis_duration`, `publish_cmd`, `publish_evt`, `subscribe_evt_all`, worker handles `AnalyzeTrack`.

- [ ] **Step 1:** Implement session + worker; test publish analyze → recv TrackAnalyzed
- [ ] **Step 2:** Commit

### Task 3: Tauri bridge

**Files:**
- Create or extend: `apps/gui-app/src-tauri/src/library_bus.rs` (or `bus_bridge` sibling)
- Modify: `apps/gui-app/src-tauri/src/lib.rs`, `Cargo.toml`

**Produces:** `library_publish`, `LibraryEvtForwarder` on `library://bus`, session in AppState at startup, remove `analyze_library_track`, sync analysis duration on settings save.

- [ ] **Step 1:** Wire host bridge; unregister analyze invoke
- [ ] **Step 2:** Commit

### Task 4: Frontend transport + UI

**Files:**
- Create: `apps/gui-app/src/lib/library/wire.ts`
- Modify: `apps/gui-app/src/lib/library/{transport,tauriTransport,memoryTransport,transport.test}.ts`
- Modify: `apps/gui-app/src/hooks/useLibrary.ts`, `apps/gui-app/src/components/LibraryPanel.tsx`
- Modify: `docs/deck-spec.md` (short note pointing at library bus spec)

**Produces:** publish/subscribe on transport; analyze via publish; subscribe patches tracks; memory transport for tests.

- [ ] **Step 1:** Implement FE + tests
- [ ] **Step 2:** Commit
