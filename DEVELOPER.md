# Developer guide

Practical notes for people hacking on Mixar itself. See the [website developer docs](apps/website/src/content/docs/developers) for subsystem references (MIDI mappings, [set history](apps/website/src/content/docs/developers/history/overview.mdx)). Runtime contracts and the library collections model: [`docs/tech-spec.md`](docs/tech-spec.md).

## High-level architecture

```
mixar/
├─ crates/                # Cargo workspace root (Cargo.toml)
│  ├─ audio-core/         # shared traits/types: AudioBackend, AudioSource, StreamParams, DeviceId
│  ├─ backend-null/       # deterministic backend for tests and CI
│  ├─ backend-cpal/       # CPAL implementation (native PipeWire on Linux when available)
│  ├─ engine-core/        # engine lifecycle, config, producer thread, track loading
│  │  ├─ lib.rs           # module declarations and public re-exports
│  │  ├─ config.rs        # EngineConfig and related types
│  │  ├─ engine.rs        # Engine public API
│  │  ├─ backend.rs       # backend factory (AudioBackend::list_names / new)
│  │  ├─ producer.rs      # ring buffer, MasterStreamSetup, producer thread loop
│  │  ├─ callback.rs      # ConsumerCallback (ring-buffer consumer)
│  │  └─ audio_source/    # FileAudioSource; re-exports AudioSource / LoadedAudio
│  ├─ engine-dsp/         # pure DSP: deck, mixer channel graph (no I/O)
│  │  ├─ lib.rs           # DspEngine
│  │  ├─ deck.rs          # playback/transport only
│  │  ├─ mixer_lane.rs    # graph node: deck + strip
│  │  ├─ mixer_channel.rs # per-lane strip (gain/EQ/filter/VU/fader)
│  │  └─ mixer.rs         # process lanes, crossfade-sum, bus routing
│  ├─ bus-api/            # one Event enum + every bus payload (engine + library)
│  ├─ event-bus/          # typed cmd/evt pair with per-subscriber merge policies
│  ├─ codec/              # decoder wrapper (symphonia)
│  ├─ resampler/          # resampler trait + rubato impl (pluggable)
│  ├─ stretch/            # realtime time-stretch for key lock (timestretch)
│  ├─ library-core/       # Library traits + Collection/Track types
│  ├─ fs-browser/         # volume listing + directory browse for audio pickers
│  ├─ library/            # library manager (canonical writable store)
│  ├─ library-adapters/   # third-party formats (Mixxx import, CDJ USB export, …)
│  ├─ analyzer-core/      # offline analysis traits and types
│  ├─ analyzer-qmdsp/     # pure-Rust qm-dsp (Mixxx) beat/key backend
│  ├─ analyzer/           # decode + analyze_file facade
│  ├─ analyzer-stems/     # stem separation pipeline
│  ├─ controller/         # MIDI / controller mapping host
│  └─ host-flutter/       # FRB Rust host over engine + library + controller
├─ apps/gui-flutter/      # Flutter desktop UI (FRB client of crates/host-flutter)
└─ samples/               # sample audio for local demos
```

### Data flow

```
AudioSource (e.g. FileAudioSource)
        │ load() → LoadedAudio
        ▼
   Engine::load_track → Deck (dry playback) + MixerChannel (auto gain)
        │  (deck resamples at playback; channel applies gain/EQ/filter/fader)
        ▼
Producer thread ──► ring buffer ──► audio callback (backend)
   (DspEngine::process)              (ConsumerCallback)
```

**Design principles:**

- All crates are small and focused.
- `engine-dsp` is pure Rust and has zero I/O dependencies (no filesystem, network, codec, or backend imports).
- `audio-core` defines the runtime trait boundary that backends implement, plus `AudioSource` / `LoadedAudio`.
- Track loading is pluggable via `AudioSource`. Concrete I/O loaders (e.g. `FileAudioSource`) live outside `engine-dsp`.
- There is no separate `backend-pipewire` crate; use `backend-cpal` for native PipeWire on Linux.

## Application logging

The Flutter desktop host and Rust crates share [`tracing`](https://docs.rs/tracing) on the Rust side. Dart/Flutter UI logging is still lightweight (console / `debugPrint`); there is no unified LogTape-style frontend pipeline yet.

### Stack

| Layer | Library | Role |
| --- | --- | --- |
| Rust crates / `host-flutter` | [`tracing`](https://docs.rs/tracing) | Facade in engine/library/controller crates; macros `tracing::{error,warn,info,debug,trace}!` |
| Host init (`init_app`) | [`tracing-subscriber`](https://docs.rs/tracing-subscriber) + [`tracing-log`](https://docs.rs/tracing-log) | stderr + the app-support `mixar.log`, attached at init via a tee writer; `EnvFilter` (default `info`, override with `RUST_LOG`); bridges leftover `log` crate calls from dependencies |
| Flutter / Dart | console / `debugPrint` | UI diagnostics during development |

### Where app data lives

Bundle / application id: `top.mixar.app` (Flutter desktop / app-support directory).

Library DB and settings sit next to each other under the platform application-support directory, resolved in Rust by `crates/host-flutter/src/app_paths.rs` (Dart passes no paths):

| Platform | Directory (typical) |
| --- | --- |
| Linux | `$XDG_DATA_HOME/top.mixar.app` or `~/.local/share/top.mixar.app` |
| macOS | `~/Library/Application Support/top.mixar.app` |
| Windows | `%APPDATA%\Mixar` |

Those rules reproduce what `path_provider` used to derive on the Dart side, so an existing install keeps finding its data; `app_paths` tests pin each rule plus the platform metadata (`APPLICATION_ID`, `PRODUCT_BUNDLE_IDENTIFIER`, `Runner.rc` `ProductName`/empty `CompanyName`).

Files of interest: `library.db`, `settings.json`, `mixar.log` (Rust diagnostics; attached by `init_app` at launch, before Dart runs).

### Raising verbosity

- **Rust:** set `RUST_LOG` (e.g. `RUST_LOG=debug`, `RUST_LOG=engine_core=debug,controller=info`) when launching the app; prefer temporary `tracing::debug!` in the crate under investigation over inventing a second logging stack.
- **Flutter:** use `debugPrint` / DevTools; avoid noisy production `print` in hot paths.

### Errors crossing the bridge

Anything returned to Dart as `Err` goes through `BridgeResult::bridge()` (`crates/host-flutter/src/bridge.rs`), which logs the cause at `ERROR` and returns the same message the UI shows — write `.bridge()?`, not `.map_err(|e| e.to_string())?`, so failures reach `mixar.log` without a second log line per call site. Panics never reach that code; `crates/host-flutter/src/api/handler.rs` supplies `FLUTTER_RUST_BRIDGE_HANDLER` with an `ErrorListener` that logs them (the generated default listener discards them).

### Notes

Controller MIDI/Rhai failures are written through `tracing` (and therefore into `mixar.log` once attached). Log rotation / shared Dart categories remain a follow-up — do not reintroduce a Tauri/LogTape pipeline or a parallel facade for first-party Mixar code.

## Codec & resampling

- **Codec:** [`symphonia`](https://crates.io/crates/symphonia) via the `codec` crate (`read_frames`, `load_entire_file`). Do not expose symphonia types in the public API. Consumers are `AudioSource` implementations (e.g. `FileAudioSource`). Never call codec from `engine-dsp`.
- **Resampler:** [`rubato`](https://crates.io/crates/rubato) behind the `resampler` crate’s `Resampler` trait (pluggable for A/B tests). Decks in `engine-dsp` resample source audio to the engine/stream sample rate **during playback**; tracks stay at native rate after `AudioSource::load()`.

## Audio backends

Backends are compiled in (Cargo features) and selected by `AudioBackend::new(name)`; no dynamic loading. The desktop app always boots the compiled-in real backend — `null` is only reachable from headless test configs.

| Backend | Role |
| --- | --- |
| `backend-cpal` | Default path. Native PipeWire on Linux when available. Optional `backend-cpal` feature on `engine-core` (default-on). Preferred under `backend = "auto"`. |
| `backend-null` | Deterministic timing for tests/CI; no real device. |

There is no `backend-pipewire` crate. Platform-native exclusive backends (ASIO / WASAPI exclusive / CoreAudio low-latency): [#453](https://github.com/geovannimp/mixar/issues/453).

## WASM / web

Not implemented. `engine-dsp` stays I/O-free so a future WASM build remains feasible. Plan and approach: [#59](https://github.com/geovannimp/mixar/issues/59).

## Performance targets

Default config: `buffer_size = 512`; each bus follows its device's mix rate unless it sets an explicit rate, and a bus sharing a device with an explicitly configured bus inherits that rate (one stream runs at one rate). Buffer duration ≈ 10.67 ms at 48 kHz.

Under a typical Linux x86_64 machine with two decks playing stereo files:

- No audible glitches (no persistent xruns) under normal conditions.
- Callback worst-case execution time under 50% of buffer duration (≈ 5.3 ms at default).
- Underruns write silence; producer warmup + ring-buffer prefill cover startup.

Always measure latency-sensitive paths with `--release`. For distribution, set explicit `target-cpu` or build per-target. Document realtime scheduling for users (`chrt` / `setcap`) when they need real-time priority.

## Release profile tweaks

```toml
[profile.release]
opt-level = 3      # "z" disables loop vectorization
lto = true
codegen-units = 1
panic = "abort"
```

## Design tokens

Colours are authored once, in DTCG JSON under `tokens/` — `primitives.json` (the ten Tailwind-aligned ladders the five selectable accent palettes are cut from), `semantic.json` (light/dark roles shared by app and website) and `web.json` (website aliases and web-only roles). [`packages/style-generator/`](packages/style-generator) turns them into three generated files; edit the JSON, never the output:

| Generated | Emitted by | Read by |
| --- | --- | --- |
| `apps/gui-flutter/lib/core/theme/mixar_palette.dart` | `packages/style-generator/dart.mjs` | app widgets, `MixarAccentPalette` / `MixarColorScale` |
| `apps/gui-flutter/lib/core/theme/mixar_colors.dart` | `packages/style-generator/dart.mjs` | `context.theme.colors` |
| `apps/website/src/styles/tokens.css` | `packages/style-generator/css.mjs` | `global.css` (imported at the top) |

- `@mixar/style-generator` is a workspace package that owns Style Dictionary (`style-dictionary`, `typescript` and `@types/node` are its dependencies, not the root's) plus the config and the three emitters. Paths in the config resolve from `import.meta.url`, so the task works from any cwd.
- `npm run tokens:build` → `moon run style-generator:generate` writes the files.
- `npm run tokens:check` → `moon run style-generator:check` typechecks the JSDoc annotations (`tsc -p .` with `checkJs`), regenerates, and fails when a committed file no longer matches its tokens — the `Check design tokens are generated` step in `lint.yml`. The `tsc` half exists because the JSDoc types are the only type system left once the config is JS; unchecked they drift into decoration, and it covers just these files in about a second.
- The generators are plain ESM JavaScript with `// @ts-check` and JSDoc types (`/** @type {import('style-dictionary/types').Config} */`, `@typedef`s for `Dictionary`/`TransformedToken`). No build step — Node runs the files as-is.
- Behaviour stays hand-written: `MixarColors.hover()`/`disable()`/`withAccentScale`, the Tailwind `@theme inline` bridge, `.band-teal`'s variable remap, gradients/`color-mix()`, shadows, and the `--stem-*` hexes (owned by `crates/codec`).

Rationale, the unification rules (light brand is teal-700 for AA; dark hairline is solid zinc-800 on both surfaces) and the full colour catalogue: [`DESIGN.md`](DESIGN.md).

## CI, testing & maintainability

Root `npm` scripts / moon drive affected checks (see [`README.md`](README.md)). Rust gates typically include `cargo fmt -- --check`, `cargo clippy -- -D warnings`, and `cargo nextest run` (Rust tests run under [cargo-nextest](https://nexte.st/) — install once with `cargo install cargo-nextest --locked`; doctests are not run by nextest, use `cargo test --doc`). Prefer `backend = "null"` for headless runs. Optional `cargo bench` / criterion for DSP hot paths ([#245](https://github.com/geovannimp/mixar/issues/245)).

### Flutter test selection

`npm run test` always runs the whole Flutter suite, and CI does the same. For local
iteration, `npm run test:affected` (task `gui-flutter:test-affected`,
`apps/gui-flutter/scripts/test_impacted.dart`) runs only the suites that transitively
import the changed files:

- `.dart` under `lib/` or `test/` is resolved through the import graph, so a change to a
  widely-reached module (e.g. `engine_providers.dart`) still runs every suite that reaches
  it, and a change to `test/support/mixar_material_app.dart` runs every suite importing it.
- `pubspec.yaml` / `pubspec.lock` are the **only** inputs that force the full suite.
- `flutter_rust_bridge.yaml`, `rust_builder/**` and `crates/host-flutter/**` expand to the
  regenerated `lib/src/rust/**` surface.
- Everything else — platform glue, assets, other crates, docs, CI config — has no
  Flutter-test impact, because the Dart suite never loads the Rust dylib (FRB opaque
  transports are faked, see `test/support/`) and never loads assets.

It diffs the branch against `origin/main` plus staged / unstaged / untracked changes, so
it is already correct before the first commit. A new file that nothing imports yet selects
zero suites. Pass `--verbose` (`mise exec -- dart run scripts/test_impacted.dart
--verbose`) to see how each changed file was classified; drop `--run` to list the suites
without running them.

Rules of thumb:

- Keep each crate small with a single responsibility.
- Keep `engine-core` split across modules (`config`, `engine`, `backend`, `producer`, `callback`, …); do not collapse into a single `lib.rs`.
- Stable `audio-core` trait surface; minimize breaking changes.
- `apps/gui-flutter` is the reference desktop host.
