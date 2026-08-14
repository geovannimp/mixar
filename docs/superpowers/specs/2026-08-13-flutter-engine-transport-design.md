# Flutter EngineTransport + EngineBuses (EngineSession shim)

Date: 2026-08-13  
Status: accepted (user waived review gates until PR)  
Depends: Library buses (`2026-08-13-flutter-library-transport-parity-design.md`), engine event bus (`2026-07-26-engine-event-bus-design.md`)

## Goal

1. Collapse bus ownership out of `EngineSession` into **host-created `EngineBuses` + `Engine`**, matching `LibraryManager::set_buses`. Keep a thin deprecated `EngineSession` shim so Tauri keeps compiling.
2. Flutter FRB **host API** (no mixer/settings UI): opaque `EngineTransport` (start/stop, Play/Pause, load-to-deck, typed evt stream) and `AudioBackendTransport` (device listing only). Dart never sees MessagePack.

## Decisions

| Topic | Choice |
|--------|--------|
| Scope | Host API + tests; mixer widgets stay placeholders; no settings UI |
| Bus ownership | Host creates cmd+evt once; injects clones into `Engine` via `set_buses` |
| Worker | `spawn_engine_worker(Arc<Mutex<Option<Engine>>>)` — `JoinHandle` stays host-owned (not inside the engine mutex) |
| `EngineSession` | Deprecated shim over engine + buses + worker; same public methods for Tauri |
| Start args | `EngineTransport::start(library, config)` — backend/sample-rate/buffer live on config (`Engine::new(config)`), not separate start parameters |
| AudioBackendTransport | Settings discovery only: `listNames` / `open(name)` / `listOutputDevices`. Not passed into start |
| Flutter bus wire | Typed FRB only (Play/Pause cmds; host-side LoadPath/LoadLibraryTrack; Status/Updated/Position/Levels/Error/Notice evts) |
| Load-to-deck | Host prepares `PreparedTrackPlayback` **outside** the engine lock (same rule as Tauri), then `load_prepared_track` |
| High-rate evts | Flutter forwarder coalesces Position/Levels/Updated/Status like Tauri `EvtForwarder` |

## Architecture

```text
Host (EngineTransport / EngineSession shim)
  EngineBuses::new() → { cmd, evt, revision }
  Engine::new(config) / new_with_library_bus → set_buses(clones)
  Arc<Mutex<Option<Engine>>>
  spawn_engine_worker(arc)  // JoinHandle on host
  share EngineBuses clones with controller later (no engine lock to publish)

Engine
  DSP + backend-from-config + Option<EngineBuses> + optional library
  publish_cmd / publish_evt / subscribe_evt_*
  no JoinHandle

Control thread (existing)
  clones buses from engine at spawn; drains cmd; locks engine; publishes evt
```

### `EngineBuses` (engine-core)

Cloneable handle bundle, same shape as `LibraryBuses` (minus analysis duration):

- `cmd: EngineBus`, `evt: EngineBus`
- `revision: Arc<AtomicU64>`
- helpers: `publish_cmd`, `publish_evt`, `subscribe_evt_all`

`Engine::set_buses(&mut self, buses: EngineBuses)` stores clones used by engine methods and by the control thread.

### Flutter `AudioBackendTransport`

Opaque FRB type wrapping `AudioBackend::new(name)`. Methods:

| Method | Notes |
|--------|--------|
| `list_names()` | `"auto"` first, then `AudioBackend::list_names()` |
| `open(name)` | `AudioBackend::new` |
| `list_output_devices()` | Real device capabilities from CPAL/backend (`id`, `name`, `is_default`, `max_channels`, `default_sample_rates`) |

### Flutter `EngineTransport`

Opaque FRB type owning:

- `Arc<Mutex<Option<Engine>>>`
- `EngineBuses`
- `Arc<Mutex<LibraryManager>>` (for prepare-outside-lock)
- control-thread `JoinHandle` + shutdown flag (Drop joins)
- evt forwarder (Drop stops)

| Method | Notes |
|--------|--------|
| `start(library, config)` | `Engine::new_with_library_bus` + `set_buses` + spawn control + `engine.start()`. Config maps onto `EngineConfig` (backend/sample_rate/buffer_size; rest default) |
| `stop()` | `engine.stop()`; Drop of transport joins control thread |
| `is_running()` | Session held and engine started |
| `play(deck_id)` / `pause(deck_id)` | typed → `publish_cmd` Play/Pause |
| `load_library_track(deck_id, track_id)` | prepare outside engine lock → `load_prepared_track` → publish Updated |
| `load_path(deck_id, path)` | same, via `prepare_file_path_for_playback` |
| `subscribe_events(sink)` | FRB `StreamSink` of thin typed evt; coalesces replaceable kinds |

Thin evt: struct + unit kind (no freezed), fields optional by kind (`running`, `deck_id`, `playing`, `track`/`track_id`, `position_ms`, peaks, `message`).

Free functions `list_backend_names` / `list_output_devices` / `start_engine` / `stop_engine` / `engine_is_running` are removed.

## Non-goals

- Mixer/settings Dart UI
- Full cmd Kind taxonomy on FRB
- Full `EngineStatus` / `DeckSnapshot` dump to Dart
- Deleting `EngineSession` from Tauri call sites (shim only)
- Passing `AudioBackendTransport` into `Engine::new` (engine still constructs its backend from config)
- Sampler-bank follow-up after load (Tauri does this; skip)

## Acceptance

- [ ] `Engine` can `set_buses` and `publish_evt` / `publish_cmd` without `EngineSession`
- [ ] `spawn_engine_worker` + Drop/shutdown works; existing engine-core bus tests still pass via shim
- [ ] Flutter `EngineTransport` start/stop/play/pause/load + event stream
- [ ] `AudioBackendTransport` lists names + null devices
- [ ] `cargo test -p engine-core` and `cargo test -p host_flutter` pass
- [ ] FRB regenerated; mixer UI still placeholder
