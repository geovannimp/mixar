# ControllerEngine Host Design

**Date:** 2026-08-04  
**Status:** accepted  
**Branch context:** `feat/controller-mapping`  
**Related:** [controller mapping](./2026-08-02-controller-mapping-design.md), [qualified controller actions](./2026-08-03-qualified-controller-actions-design.md), [#133 always-allow](https://github.com/geovannimp/rust-dj-engine/issues/133)

## Goal

Wire MIDI controllers into the desktop (and later WASM) app through a **`ControllerEngine`** in the `controller` crate: midir + app-data mapping bundles + lifecycle API. Tauri only glues that engine to EngineSession, LibrarySession, and the FE.

## Decisions

| Topic | Choice |
|-------|--------|
| Crate layout | Single `controller` crate: `MappingSession` + `ControllerEngine` + midir |
| MIDI backend | midir (native ALSA/etc.; `wasm32` Web MIDI). Git-pin until crates.io ships alsa `<0.13` (see [midir#199](https://github.com/Boddlnagg/midir/pull/199)) so Linux links with cpal |
| Mapping storage | `app_data_dir/mappings/<id>/` (same root as `library.db`) |
| Seed policy | **Copy if missing** from shipped `mappings/` |
| Update | Settings: **Update all** + **per-mapping update** (overwrite app-data from shipped) |
| User tree | None |
| Connect policy | Match on connect → ask FE every time → Enable attaches; Dismiss ignores. No remembered allow (follow-up [#133](https://github.com/geovannimp/rust-dj-engine/issues/133)) |
| Publish | Host `ActionPublish`: engine cmds (shared host path with `engine_publish`) + library nav `publish_evt` |
| LEDs | When attached, engine evts → mapping outputs → midir out |

## Architecture

```text
shipped mappings/  --copy-if-missing / update-->  app-data/mappings/
                                                      │
midir  <──►  ControllerEngine  <── MappingSession
                 │ list_devices, list_mappings
                 │ enable/disable, update_*, pump
                 │ MappingOffer / DeviceGone events
                 ▼
Tauri host ── ActionPublish ──► EngineSession + LibrarySession
                 │
                 ▼
               FE (prompt + Controllers settings)
```

## ControllerEngine API (v1)

Construct with `app_mappings_dir` + `shipped_mappings_dir`.

| API | Behavior |
|-----|----------|
| `ensure_seeded()` | Copy each shipped `<id>` into app-data if missing |
| `update_mapping(id)` / `update_all_mappings()` | Overwrite app-data from shipped; reload if attached |
| `list_mappings()` | Catalog from app-data (`id`, name, identity hints, attached?) |
| `list_devices()` | midir ports + best matching mapping id if any |
| `enable_mapping(id)` | Attach session to matching live port (from prompt or settings) |
| `disable_mapping(id)` | `on_shutdown`, drop attach |
| `poll_devices()` | Diff ports → emit connect offers / disconnect |
| `pump(bus)` | Drain MIDI → `handle_midi`; optional LED path from host-fed play state |

Host events (channel or callback): `MappingOffer { mapping_id, port_name, device_name }`, `MappingAttached`, `MappingDetached`, `DeviceGone`.

## Tauri glue

- Setup: seed mappings, start background poll+pump, manage `ControllerEngine`
- Invokes: list/update/enable/disable
- Emit: mapping offer → FE dialog/toast
- `ActionPublish` shares load/sampler/`StartEngine` handling with `engine_publish`
- Engine down: drop engine cmds (log); library nav still publishes

## Settings UI

New **Controllers** section: mapping list, enable/disable when a port matches, Update / Update all, device list for debug.

## Out of scope

- [#133](https://github.com/geovannimp/rust-dj-engine/issues/133) persist / Always allow
- MIDI learn, multi-controller, marketplace
- Replacing `midi-map-probe` (keep as standalone debug; may share git midir)

## Acceptance

- [x] gui-app links `controller` + cpal (single `alsa-sys`) via git midir
- [x] App-data mappings seeded copy-if-missing; update all / per-id from settings
- [x] Connect match → FE prompt → enable attaches → play/volume-class cmds + library nav
- [x] Disable / disconnect detaches cleanly; reconnect asks again
- [ ] LEDs update when attached (best-effort play signal) — API present (`on_deck_playing`); host evt hook deferred
