# Controller Mapping Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship v1 of the open MIDI controller mapping API (`controller` crate + map-check + schemas + example bundle + host MidiPort adapter) per `docs/superpowers/specs/2026-08-02-controller-mapping-design.md`.

**Architecture:** Host-agnostic `controller` crate loads `device.toml` / `map.toml` / optional `script.rhai`, resolves aliases, applies declarative map (+ modifiers / soft-takeover), runs thin Rhai hooks, and publishes/subscribes via caller-supplied bus + MIDI traits. No OS MIDI inside the core crate; `midir` lives behind a `native-midi` feature / host adapter. WASM-safe default features.

**Tech Stack:** Rust workspace crate, `toml` + `serde`, `rhai` (sync), `engine-api`, optional `midir`, moon tasks on `crates` project, TOML Schema `.tosd` files for editors (semantic validation in Rust; do not require unpublished crates.io `toml-schema` at runtime).

## Global Constraints

- Follow `docs/superpowers/specs/2026-08-02-controller-mapping-design.md` exactly for bundle format, modifier priority, soft-takeover (3/127), alias rules, and errors.
- Mapper publishes normal `Origin::Deck(n)` / `Mixer` / `Engine` — same path as UI.
- No MIDI on the audio callback.
- Cargo via `cargo --manifest-path crates/Cargo.toml …`.
- YAGNI: no learn UI, no Mixxx/VDJ importers, no marketplace, no HID.
- Actions must map only to existing `engine_api::Kind` + `CmdBody` variants.
- GPL-3.0 workspace license; match existing crate `Cargo.toml` metadata style.

## File structure

| Path | Responsibility |
|------|----------------|
| `crates/controller/Cargo.toml` | New workspace member |
| `crates/controller/src/lib.rs` | Public API re-exports |
| `crates/controller/src/error.rs` | `LoadError` / `RuntimeError` |
| `crates/controller/src/midi.rs` | Short MIDI parse + identity |
| `crates/controller/src/catalog.rs` | Closed alias + action vocabularies |
| `crates/controller/src/device.rs` | `device.toml` types + parse |
| `crates/controller/src/map_file.rs` | `map.toml` types + parse |
| `crates/controller/src/bundle.rs` | Load/validate bundle dir |
| `crates/controller/src/action.rs` | Action string → `(Origin, Kind, CmdBody)` |
| `crates/controller/src/session.rs` | Runtime: input, modifiers, soft-takeover, outputs, snapshot |
| `crates/controller/src/script.rs` | Rhai engine + host API |
| `crates/controller/src/check.rs` | `map-check` validation entry used by bin + tests |
| `crates/controller/src/bin/map-check.rs` | CLI |
| `crates/controller/src/native_midi.rs` | Optional midir adapter (`native-midi` feature) |
| `schemas/device.tosd` | Editor schema |
| `schemas/map.tosd` | Editor schema |
| `mappings/example-generic/` | Example bundle |
| `crates/Cargo.toml` | Add member + shared deps if needed |
| `crates/moon.yml` | `test-mappings` / `test-mapping` tasks |
| `apps/gui-app/src-tauri/...` | Thin host wiring only if minimal; prefer crate-level native adapter + note for Tauri follow-up if GUI wiring bloated |

---

### Task 1: Scaffold `controller` crate

**Files:**
- Create: `crates/controller/Cargo.toml`
- Create: `crates/controller/src/lib.rs`
- Create: `crates/controller/src/error.rs`
- Modify: `crates/Cargo.toml` (members + optional workspace deps `rhai`)
- Test: `crates/controller/tests/smoke.rs`

**Interfaces:**
- Produces: empty crate that compiles; `pub fn crate_name() -> &'static str` temporary or just `pub use error::LoadError`

- [ ] **Step 1: Add workspace member and crate files**

`crates/controller/Cargo.toml`:

```toml
[package]
name = "controller"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "MIDI controller mapping bundles (device/map/Rhai) for the engine bus"

[features]
default = []
native-midi = ["dep:midir"]

[dependencies]
engine-api = { path = "../engine-api" }
serde = { workspace = true, features = ["derive"] }
toml = { workspace = true }
thiserror = { workspace = true }
rhai = { version = "1.21", default-features = false, features = ["std", "sync"] }
midir = { version = "0.10", optional = true }

[dev-dependencies]
```

Add `"controller"` to workspace `members` in `crates/Cargo.toml`. Add `rhai` under `[workspace.dependencies]` if preferred.

- [ ] **Step 2: Smoke test**

```rust
#[test]
fn controller_crate_links() {
    let _ = controller::LoadError::Schema { version: 99 };
}
```

- [ ] **Step 3: Run test**

Run: `cargo test --manifest-path crates/Cargo.toml -p controller`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add crates/controller crates/Cargo.toml
git commit -m "feat(controller): scaffold mapping crate"
```

---

### Task 2: Parse + validate `device.toml` / `map.toml`

**Files:**
- Create: `crates/controller/src/midi.rs`, `catalog.rs`, `device.rs`, `map_file.rs`, `bundle.rs`
- Test: `crates/controller/tests/bundle_load.rs`
- Create fixtures under `crates/controller/tests/fixtures/valid-minimal/` and `invalid-unknown-alias/`

**Interfaces:**
- Produces:
  - `pub struct Bundle { pub device: DeviceFile, pub map: MapFile, pub script_source: Option<String>, pub root: PathBuf }`
  - `pub fn load_bundle(dir: &Path) -> Result<Bundle, LoadError>`
  - Validates schema_version == 1, closed input keys, custom-only free names for modifiers, input MIDI clash among in/inout aliases, map action names known, modifier/output alias refs resolve

- [ ] **Step 1: Failing test — valid minimal loads**

```rust
#[test]
fn loads_valid_minimal_bundle() {
    let b = controller::load_bundle(Path::new("tests/fixtures/valid-minimal")).unwrap();
    assert_eq!(b.device.id, "test.minimal");
}
```

- [ ] **Step 2: Implement parse/validate until PASS; add failing unknown-alias test then make it return `LoadError`**

- [ ] **Step 3: Commit**

```bash
git commit -m "feat(controller): load and validate mapping bundles"
```

---

### Task 3: Actions + session input path (modifiers, soft-takeover)

**Files:**
- Create: `crates/controller/src/action.rs`, `session.rs`
- Test: `crates/controller/tests/session_input.rs`

**Interfaces:**
- Produces:
  - `pub trait BusPublish { fn publish(&mut self, origin: Origin, kind: Kind, body: CmdBody); }`
  - `pub struct MappingSession { ... }`
  - `impl MappingSession { pub fn from_bundle(bundle: Bundle) -> Result<Self, LoadError>; pub fn handle_midi(&mut self, bytes: &[u8], bus: &mut impl BusPublish); pub fn set_control_value(&mut self, origin: Origin, key: &str, value: f32); /* for soft-takeover mirror */ }`
  - Modifier: active `custom.*` note/cc>0 wins over unmodified binding
  - Soft-takeover threshold: `3.0/127.0`
  - CC coalesce: max one publish per alias per 1/60s (simple Instant gate)

- [ ] **Step 1: Test note → toggle_play publishes Play or Pause based on snapshot playing flag (default false → Play)**

- [ ] **Step 2: Test modifier shift selects alternate action**

- [ ] **Step 3: Test soft-takeover blocks then latches**

- [ ] **Step 4: Implement + commit**

```bash
git commit -m "feat(controller): map MIDI inputs to engine commands"
```

---

### Task 4: Outputs from engine snapshot

**Files:**
- Modify: `session.rs`
- Test: `crates/controller/tests/session_output.rs`

**Interfaces:**
- Produces:
  - `pub trait MidiOut { fn send(&mut self, bytes: &[u8]); }`
  - `MappingSession::on_deck_playing(&mut self, deck: u16, playing: bool, midi: &mut impl MidiOut)` or generic `apply_output_signal(section, alias, active: bool, ...)`
  - Resolve `on`/`off` as alias string or inline MIDI table → bytes

- [ ] **Step 1: Test play_pause output fires pause_led alias bytes when playing true**

- [ ] **Step 2: Implement + commit**

```bash
git commit -m "feat(controller): map engine state to MIDI outputs"
```

---

### Task 5: Rhai hooks

**Files:**
- Create: `crates/controller/src/script.rs`
- Modify: `session.rs` / `bundle.rs`
- Test: `crates/controller/tests/script_hooks.rs`

**Interfaces:**
- Produces: compile `script.rhai` at load; call `on_init` / `on_shutdown`; `script = "fn"` bindings; host fns `publish` / `midi_out` / `modifier_active` registered on engine
- Runtime script errors → return/log `RuntimeError`, do not panic

- [ ] **Step 1: Test on_init calls midi_out**

- [ ] **Step 2: Implement + commit**

```bash
git commit -m "feat(controller): add Rhai mapping hooks"
```

---

### Task 6: map-check CLI, moon tasks, schemas, example bundle

**Files:**
- Create: `crates/controller/src/check.rs`, `crates/controller/src/bin/map-check.rs`
- Create: `schemas/device.tosd`, `schemas/map.tosd`
- Create: `mappings/example-generic/{device.toml,map.toml}`
- Modify: `crates/moon.yml`
- Test: map-check on example + invalid fixture

**Interfaces:**
- CLI: `map-check <dir>` or `map-check --all <mappings-root>`
- Moon:
  - `test-mappings`: `cargo run -p controller --bin map-check -- --all ../mappings` (cwd crates)
  - `test-mapping`: accepts args after `--` for one id

- [ ] **Step 1: Implement check + example + schemas + moon tasks**

- [ ] **Step 2: Run `moon run rust:test-mappings` (or cargo equivalent) — PASS**

- [ ] **Step 3: Commit**

```bash
git commit -m "feat(controller): add map-check, schemas, and example mapping"
```

---

### Task 7: Native MIDI adapter + integration test

**Files:**
- Create: `crates/controller/src/native_midi.rs` (feature-gated)
- Test: `crates/controller/tests/integration_fake_port.rs` (fake MidiPort, no midir required)

**Interfaces:**
- `pub trait MidiPort { fn send(&mut self, bytes: &[u8]) -> Result<(), MidiPortError>; }` — input delivered via `session.handle_midi`
- Integration: fake in bytes → BusPublish captured cmds; playing flip → out bytes

- [ ] **Step 1: Integration test PASS without hardware**

- [ ] **Step 2: Optional midir enumerate helper behind `native-midi` compiles**

- [ ] **Step 3: Commit**

```bash
git commit -m "feat(controller): add MIDI port trait and integration test"
```

---

### Task 8: Spec status + PR hygiene

**Files:**
- Modify: `docs/superpowers/specs/2026-08-02-controller-mapping-design.md` status → `implemented (v1 runtime)` when acceptance met
- Modify: `docs/deck-spec.md` §5.16 / Phase 5 note if a one-line pointer helps (only if already linking specs)

- [ ] **Step 1: Run full `cargo test -p controller` and `map-check --all`**

- [ ] **Step 2: Update spec status checkbox acceptance items that are done**

- [ ] **Step 3: Open PR against main**

---

## Spec coverage checklist

| Spec item | Task |
|-----------|------|
| Bundle device/map/script | 2, 5 |
| Closed catalog + custom | 2 |
| Modifiers + soft-takeover | 3 |
| Outputs alias or inline | 4 |
| Rhai hooks | 5 |
| map-check + moon | 6 |
| TOML Schema .tosd | 6 |
| Example bundle | 6 |
| Host MidiPort / WASM-safe core | 1, 7 |
| Autoload by identity | 2 (`DeviceFile` match helpers) + 7 docs; full Tauri hotplug can be thin follow-up if not in gui this PR |

## Self-review notes

- No TBD placeholders in tasks.
- `set_filter` used in modifier examples (exists as `Kind::SetFilter`).
- `.tosd` shipped for editors; runtime validation is Rust (toml-schema crate not required on crates.io for v1).
