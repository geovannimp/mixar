# Controller TOML Schema Pointers Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (inline). Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `[toml-schema]` pointers to mapping TOMLs, rewrite `.tosd` files, and serde-ignore the reserved table at load time.

**Architecture:** Optional `TomlSchemaRef` on `DeviceFile`/`MapFile`; structural `.tosd` under `schemas/`; relative `location` from each data file. Runtime validation stays Rust.

**Tech Stack:** Rust `serde`/`toml`, TOML Schema `.tosd`, existing `controller` fixtures + `map-check`.

## Global Constraints

- No toml-schema CLI/crate in CI this pass
- Relative paths only (GitHub URLs later)
- App `schema_version = 1` unchanged
- Closed catalogs stay in Rust, not `.tosd` `allowedvalues`

---

### Task 1: Serde-ignore `[toml-schema]`

**Files:**
- Modify: `crates/controller/src/device.rs`
- Modify: `crates/controller/src/map_file.rs`
- Test: `crates/controller/tests/bundle_load.rs` (or new unit in device parse)

**Interfaces:**
- Produces: `TomlSchemaRef { version: Option<String>, location: Option<String> }` on both parsers via `#[serde(default, rename = "toml-schema")] toml_schema: Option<TomlSchemaRef>`

- [ ] **Step 1:** Add failing test: parse minimal device/map TOML that includes `[toml-schema]` with version+location; expect Ok
- [ ] **Step 2:** Add `TomlSchemaRef` + field on `DeviceFile` and `MapFile`
- [ ] **Step 3:** Run `cargo test -p controller --manifest-path crates/Cargo.toml`
- [ ] **Step 4:** Commit

### Task 2: Rewrite `.tosd` + add pointers

**Files:**
- Rewrite: `schemas/device.tosd`, `schemas/map.tosd`
- Modify: all 6 `device.toml` + 6 `map.toml` under `mappings/` and `crates/controller/tests/fixtures/`
- Modify: `crates/controller/tests/engine_seed.rs` writer if it emits device/map TOML
- Docs: one-line pointer in `docs/superpowers/specs/2026-08-02-controller-mapping-design.md`

- [ ] **Step 1:** Write structural `device.tosd` / `map.tosd` per design
- [ ] **Step 2:** Insert `[toml-schema]` blocks with correct relative paths
- [ ] **Step 3:** `cargo test -p controller` + `cargo run -p controller --bin map-check -- --all ../mappings`
- [ ] **Step 4:** Commit, resolve PR thread `PRRT_kwDORVdnwc6XGrHk`
