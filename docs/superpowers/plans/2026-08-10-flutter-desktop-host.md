# Flutter Desktop Host Smoke — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Scaffold an experimental Flutter host (`apps/gui-flutter`) + FRB Rust crate (`crates/host-flutter`) that lists audio backends/devices and starts/stops the engine on Linux.

**Architecture:** Thin typed FRB API in `host-flutter` over `engine-core`’s `EngineSession` / `AudioBackend`. Flutter smoke UI only. Tauri untouched. Flutter pinned via mise.

**Tech Stack:** Flutter (mise), flutter_rust_bridge v2, Rust workspace (`engine-core`, `audio-core`, default CPAL features).

## Global Constraints

- Flutter via mise only (`mise.toml`); no system Flutter / FVM
- Layout: `apps/gui-flutter` + `crates/host-flutter` (workspace member under `crates/Cargo.toml`)
- Platforms enabled: linux, macos, windows, web — verify Linux only
- Start engine idempotent if already running; errors to Dart, do not exit process
- Defaults: sample rate 48000, buffer 512, backend `"auto"` when unspecified
- No Tauri changes; no MessagePack bus; no deck/library UI
- Spec: `docs/superpowers/specs/2026-08-10-flutter-desktop-host-design.md`

---

### Task 1: Pin Flutter with mise + install FRB codegen

**Files:**
- Modify: `mise.toml`
- Create: (toolchain installs under mise home)

**Interfaces:**
- Produces: `flutter` and `dart` on PATH via `mise exec`; `flutter_rust_bridge_codegen` installed

- [ ] **Step 1: Add Flutter to mise.toml**

```toml
# mise.toml
[settings]
idiomatic_version_file_enable_tools = ["node", "rust"]

[tools]
flutter = "stable"
```

- [ ] **Step 2: Install Flutter**

Run: `mise install flutter`
Expected: Flutter SDK installed; `mise exec -- flutter --version` prints a version.

- [ ] **Step 3: Enable Linux desktop (if needed)**

Run: `mise exec -- flutter config --enable-linux-desktop && mise exec -- flutter doctor`
Expected: Linux toolchain noted; doctor may warn on optional components — OK if `flutter create` / `flutter run -d linux` can work.

- [ ] **Step 4: Install flutter_rust_bridge_codegen**

Run: `cargo install flutter_rust_bridge_codegen --locked`
Expected: `flutter_rust_bridge_codegen --version` succeeds (v2.x).

- [ ] **Step 5: Commit**

```bash
git add mise.toml docs/superpowers/specs/2026-08-10-flutter-desktop-host-design.md
git commit -m "$(cat <<'EOF'
docs: add Flutter host design; pin Flutter via mise

EOF
)"
```

---

### Task 2: Scaffold Flutter app + host-flutter crate via FRB

**Files:**
- Create: `apps/gui-flutter/**` (FRB-generated Flutter app)
- Create: `crates/host-flutter/**` (FRB-generated Rust crate)
- Modify: `crates/Cargo.toml` (add workspace member)
- Delete: `crates/host-flutter/Cargo.lock` if generated

**Interfaces:**
- Produces: FRB hello-world link between Dart and `host_flutter`

- [ ] **Step 1: Create project with external rust crate dir**

```bash
cd /home/geovanni/Projects/rust-mixer/apps
mise exec -- flutter_rust_bridge_codegen create gui_flutter \
  --rust-crate-dir ../../crates/host-flutter \
  --rust-crate-name host_flutter \
  --platforms linux,macos,windows,web
```

If the folder is `apps/gui_flutter`, rename to `apps/gui-flutter` and fix any path references in generated config (`flutter_rust_bridge.yaml`, cargokit paths) so they still resolve to `crates/host-flutter`.

- [ ] **Step 2: Join Cargo workspace**

```bash
rm -f /home/geovanni/Projects/rust-mixer/crates/host-flutter/Cargo.lock
```

Add `"host-flutter"` to `members` in `crates/Cargo.toml`.

- [ ] **Step 3: Verify generate + Linux build of stock template**

```bash
cd /home/geovanni/Projects/rust-mixer/apps/gui-flutter
mise exec -- flutter_rust_bridge_codegen generate
cargo check --manifest-path /home/geovanni/Projects/rust-mixer/crates/Cargo.toml -p host_flutter
mise exec -- flutter build linux --debug
```

Expected: generate/check/build succeed (first Linux build may take a while).

- [ ] **Step 4: Commit**

```bash
git add apps/gui-flutter crates/host-flutter crates/Cargo.toml
git commit -m "$(cat <<'EOF'
chore: scaffold Flutter host with flutter_rust_bridge

EOF
)"
```

---

### Task 3: Engine smoke API in host-flutter + Rust tests

**Files:**
- Create/Modify: `crates/host-flutter/src/api/engine.rs` (or FRB-conventional `api/*.rs`)
- Modify: `crates/host-flutter/src/api/mod.rs` / `lib.rs` as FRB expects
- Modify: `crates/host-flutter/Cargo.toml` — deps: `engine-core` (default features), `audio-core`, `anyhow`, `flutter_rust_bridge`, `lazy_static` or `std::sync::Mutex`
- Create: `crates/host-flutter/tests/smoke_null_backend.rs` OR unit tests in `src/api/engine.rs`

**Interfaces:**
- Produces (Rust, FRB-exported):
  - `list_backend_names() -> Vec<String>`
  - `list_output_devices(backend: String) -> Result<Vec<OutputDevice>>`
  - `start_engine(backend: String, sample_rate: Option<u32>, buffer_size: Option<u32>) -> Result<()>`
  - `stop_engine() -> Result<()>`
  - `engine_is_running() -> bool`
  - `struct OutputDevice { id, name, is_default, max_channels, default_sample_rates }`
- Consumes: `engine_core::{AudioBackend, EngineConfig, EngineSession, create_backend}`

- [ ] **Step 1: Add path deps to host-flutter Cargo.toml**

```toml
[dependencies]
engine-core = { path = "../engine-core" }
anyhow = { workspace = true }
# keep existing flutter_rust_bridge deps from scaffold
```

- [ ] **Step 2: Write failing smoke test (null backend)**

```rust
// crates/host-flutter/tests/smoke_null_backend.rs
#[test]
fn start_stop_null_backend() {
    // Call the same functions FRB will export (pub in api module).
    assert!(!host_flutter::api::engine::engine_is_running());
    host_flutter::api::engine::start_engine("null".into(), None, None).unwrap();
    assert!(host_flutter::api::engine::engine_is_running());
    host_flutter::api::engine::start_engine("null".into(), None, None).unwrap(); // idempotent
    host_flutter::api::engine::stop_engine().unwrap();
    assert!(!host_flutter::api::engine::engine_is_running());
}

#[test]
fn list_backends_includes_null() {
    let names = host_flutter::api::engine::list_backend_names();
    assert!(names.iter().any(|n| n == "null"));
}
```

- [ ] **Step 3: Run test — expect fail (module missing)**

Run: `cargo test --manifest-path crates/Cargo.toml -p host_flutter --test smoke_null_backend`
Expected: FAIL (unresolved module / function)

- [ ] **Step 4: Implement engine API**

Session holder:

```rust
use std::sync::Mutex;
use engine_core::{create_backend, AudioBackend, EngineConfig, EngineSession};

static SESSION: Mutex<Option<EngineSession>> = Mutex::new(None);

pub struct OutputDevice {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    pub max_channels: u16,
    pub default_sample_rates: Vec<u32>,
}

pub fn list_backend_names() -> Vec<String> {
    AudioBackend::list_names()
}

pub fn list_output_devices(backend: String) -> anyhow::Result<Vec<OutputDevice>> {
    let devices = create_backend(&backend)?.list_output_devices()?;
    Ok(devices
        .into_iter()
        .map(|d| OutputDevice {
            id: d.id.as_str().to_string(),
            name: d.name,
            is_default: d.is_default,
            max_channels: d.max_channels,
            default_sample_rates: d.default_sample_rates,
        })
        .collect())
}

pub fn start_engine(
    backend: String,
    sample_rate: Option<u32>,
    buffer_size: Option<u32>,
) -> anyhow::Result<()> {
    let mut slot = SESSION.lock().map_err(|_| anyhow::anyhow!("session lock poisoned"))?;
    if slot.is_some() {
        return Ok(());
    }
    let mut config = EngineConfig::default();
    config.backend = backend;
    if let Some(sr) = sample_rate {
        config.sample_rate = sr;
    }
    if let Some(bs) = buffer_size {
        config.buffer_size = bs;
    }
    let session = EngineSession::new(config)?;
    session.with_engine(|engine| engine.start().map_err(anyhow::Error::from))?;
    *slot = Some(session);
    Ok(())
}

pub fn stop_engine() -> anyhow::Result<()> {
    let mut slot = SESSION.lock().map_err(|_| anyhow::anyhow!("session lock poisoned"))?;
    let Some(session) = slot.as_mut() else {
        return Ok(());
    };
    session.with_engine(|engine| engine.stop().map_err(anyhow::Error::from))?;
    *slot = None;
    Ok(())
}

pub fn engine_is_running() -> bool {
    SESSION
        .lock()
        .map(|s| s.is_some())
        .unwrap_or(false)
}
```

Wire FRB annotations / module exports per the scaffold’s `api/simple.rs` pattern (`#[flutter_rust_bridge::frb(sync)]` only if sync is desired; prefer async-friendly defaults FRB generates).

Check `EngineSession::with_engine` and `Engine::stop` signatures against `crates/engine-core` and adjust.

- [ ] **Step 5: Regenerate FRB + run tests**

```bash
cd apps/gui-flutter && mise exec -- flutter_rust_bridge_codegen generate
cargo test --manifest-path crates/Cargo.toml -p host_flutter
```

Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add crates/host-flutter apps/gui-flutter
git commit -m "$(cat <<'EOF'
feat(host-flutter): engine start/stop and device listing API

EOF
)"
```

---

### Task 4: Smoke UI in Flutter

**Files:**
- Modify: `apps/gui-flutter/lib/main.dart`
- Optionally keep generated FRB entrypoint init (`RustLib.init()`)

**Interfaces:**
- Consumes: generated Dart bindings for the Task 3 API

- [ ] **Step 1: Replace main.dart with smoke screen**

Minimal Material app:

- On init: `RustLib.init()`, then `listBackendNames()`, select `"auto"` if present else first
- Dropdown for backend; button “Refresh devices” → `listOutputDevices`
- ListView of devices (`name`, `isDefault`)
- Start / Stop buttons calling `startEngine` / `stopEngine`
- Text for `engineIsRunning` and last error

Use the exact generated function names from `lib/src/rust/api/...` after codegen.

- [ ] **Step 2: Build Linux**

```bash
cd apps/gui-flutter && mise exec -- flutter build linux --debug
```

Expected: success

- [ ] **Step 3: Manual smoke (if display available)**

```bash
cd apps/gui-flutter && mise exec -- flutter run -d linux
```

Expected: UI lists backends/devices; Start/Stop toggles status. If no DISPLAY, skip and rely on Rust tests + build.

- [ ] **Step 4: Commit**

```bash
git add apps/gui-flutter
git commit -m "$(cat <<'EOF'
feat(gui-flutter): smoke UI for engine and devices

EOF
)"
```

---

### Task 5: README note + AGENTS.md workspace fact

**Files:**
- Modify: `README.md` (short “Experimental Flutter host” pointer) OR `apps/gui-flutter/README.md`
- Modify: `AGENTS.md` Learned Workspace Facts — add Flutter host paths + mise flutter

- [ ] **Step 1: Add apps/gui-flutter/README.md**

Document: mise install, `flutter_rust_bridge_codegen generate`, `flutter run -d linux`, link to design spec.

- [ ] **Step 2: Update AGENTS.md** with one bullet on `apps/gui-flutter` + `crates/host-flutter` + mise Flutter pin.

- [ ] **Step 3: Commit**

```bash
git add apps/gui-flutter/README.md AGENTS.md
git commit -m "$(cat <<'EOF'
docs: note experimental Flutter host setup

EOF
)"
```

---

## Plan self-review

1. **Spec coverage:** Layout, mise, FRB scaffold, typed API, smoke UI, Linux-only verify, Tauri untouched — all tasked.
2. **Placeholders:** None intentional; Step 4 of Task 3 notes adjusting to real `EngineSession`/`stop` signatures during implementation.
3. **Types:** `OutputDevice` and API names consistent across Tasks 3–4.
