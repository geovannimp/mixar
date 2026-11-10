# gui-flutter

Flutter desktop host for Mixar, bridged to Rust via
[flutter_rust_bridge](https://github.com/fzyzcjy/flutter_rust_bridge).

This is the **primary** desktop UI.

## Layout

| Path | Role |
|------|------|
| `apps/gui-flutter` | Flutter app (linux / macOS / windows / web / android enabled; Linux and Android verified) |
| `crates/host-flutter` | FRB Rust host (`host_flutter`) over `engine-core` + library browse |

Library browse opens `{getApplicationSupportDirectory()}/library.db` (app id `top.mixar.app`; desktop only — web shows a placeholder). UI: MixarTheme collections/drive tabs (Shad-bridged), add-folder, analyze/refresh, visible-row artwork, and a sortable track list (shared with the history entry list through one `TrackListView`, so selection, keyboard load, drag-to-deck and the row menu behave identically). In-flight work renders as a per-row overlay: one right-aligned phase pill per job lane (analysis and stem separation run independently, so a track being analysed and split at once shows both) plus a progress bar per job, rather than in the title cell — so progress ticks rebuild one row instead of the whole list. The 3-dot menu stays open during a job; each action disables itself while its own job runs. FRB `LibraryTransport` also exposes add-folder / resolve / track list + `getTrack` (artwork only on `getTrack` until covers live in `library.db`) and a thin typed analyze/refresh event stream; waveform rasterization stays in Flutter.

Generated FRB outputs under `lib/src/rust/` and `crates/host-flutter/src/frb_generated.rs` are **committed** (FRB’s usual workflow so clones build without running codegen first). Regenerate after Rust API changes.

## Prerequisites

From the repo root (Flutter + ninja are pinned in `mise.toml`):

```sh
mise install   # Flutter postinstall activates dashmonx (https://github.com/rosenpin/dashmonx)
export PATH="$HOME/.cargo/bin:$PATH"   # for flutter_rust_bridge_codegen
cargo install flutter_rust_bridge_codegen --version 2.13.0 --locked   # once, if missing / wrong version
```

## Develop

From the repo root (`dashmonx` wraps `flutter run` and hot-reloads on `lib/` changes):

```sh
npm run app:linux
# or: moon run gui-flutter:dev-linux
```

Desktop uses [`window_manager`](https://pub.dev/packages/window_manager) with `TitleBarStyle.hidden` and in-app min/max/close controls (drag empty header regions to move; double-click to maximize).

### Android

Android audio runs on cpal's AAudio host: `engineTransportProvider` starts the engine on every host with an audio output (`supportsAudioEngine` in `lib/core/utils/desktop.dart` — desktop and Android; web has no engine), so decks load, play and mix there too. The controller transport stays desktop-only (`isDesktopWindow`), so Android has no mapped controller session. First-run setup still probes MIDI once (`detectControllers`), because the setup step is built from what the OS reports.

Devices on Android are planned from what their Java audio profile advertises, and a phone or tablet speaker usually advertises PCM 16-bit only. AAudio still opens the stream as `PCM_Float` and converts, so `backend-cpal` plans an integer-only device's rates and channel counts as `f32` instead of dropping it (`CpalBackend::output_ranges`). Without that the engine fails with "No output device available" and every engine-gated surface — track drag, deck drop targets, the row menu's Load to deck — goes dead with it.

Outputs are selectable per bus in Settings → Audio. The host default is listed first and marked `is_default` (on AAudio it is a `Device(None)` alias for whatever route the OS picks — speaker, headphones, Bluetooth), and the other sinks — a controller's USB interface, a headset — open by id, which is how a DJ app targets a specific interface. Ids resolve by scanning the enumeration because cpal's AAudio host answers no `device_by_id`. Android also enumerates its telephony endpoints as outputs (8/16 kHz); those are left out of the Android list because selecting one would run the whole mix at voice grade. Desktop keeps its low-rate devices — a 32 kHz interface there is real hardware the user may own, and the engine opens it at its own rate (`resolve_plan_rate`) — but a sink that cannot reach 44.1 kHz is labelled `— low quality output` in the picker (`outputDeviceCarriesMixerRates`), so choosing it is a deliberate act rather than a mistake.

Android runs landscape-only and full-screen: the manifest pins `screenOrientation="sensorLandscape"` and `main.dart` switches to `SystemUiMode.immersiveSticky` before the first frame. Android 16+ ignores the orientation lock on large screens (≥600 dp), where the windowing system wins.

App data lands in the platform files dir (`/data/user/<id>/top.mixar.app/files/` → `library.db`, `settings.json`, `mixar.log`). Rust cannot derive it — Android app processes have no `HOME`/`XDG_DATA_HOME`, which used to resolve to `/` and fail with `I/O error: Read-only file system` — so `main.dart` passes `path_provider`'s application support directory to `setAppSupportDir` before any transport opens; `init_app` skips log attachment on Android and `setAppSupportDir` attaches it.

MIDI needs one more piece of glue the Flutter embedding does not provide: `ndk-context`. midir's Android backend and cpal's AAudio host read the JavaVM and an Application context from it, and nothing initializes it here — a Flutter app has no `android_activity` glue crate. `MainActivity.onCreate` therefore loads `libhost_flutter.so` and calls `initAndroidContext(applicationContext)` before `super.onCreate` starts Dart; the JNI export lives in `crates/host-flutter/src/android.rs` (the package's `_` is `_1` in the mangled symbol, so renaming either side renames the function). Without it the first MIDI probe panics with "android context was not initialized", and the Dart future awaiting it never completes: first-run setup stays on its spinner with nothing in `mixar.log` but that one line.

Folder imports scan with `std::fs` on the path `file_picker` returns, and Android's scoped storage hides other apps' audio from that read until the process holds the media permission: without it the picked folder lists no files, so the collection would import empty. The manifest declares `READ_MEDIA_AUDIO` (API 33+) and `READ_EXTERNAL_STORAGE` (`maxSdkVersion="32"`), and `MainActivity` hosts the `top.mixar.app/media_access` channel behind `ensureAudioReadAccess` (`lib/core/utils/android_media_access.dart`), which the library actions call before adding a folder. A refusal stops the import with an actionable message instead of creating a collection that can only list nothing; a folder that genuinely has no audio files reports that too. The permission is the OS one, so it survives restarts once granted.

One-time host setup (SDK location can also come from `ANDROID_HOME`):

```sh
flutter config --android-sdk "$HOME/Android/sdk"
mise exec -- sdkmanager "ndk;28.2.13676358"   # flutter.ndkVersion for Flutter 3.47.7
rustup target add aarch64-linux-android x86_64-linux-android
```

Then, with a device or emulator up (`flutter emulators --launch Pixel_7_Pro_API_36`, `adb devices`):

```sh
npm run app:android                   # moon run gui-flutter:dev-android
moon run gui-flutter:build-android    # debug APK in build/app/outputs/flutter-apk/
```

`build-android` passes `--target-platform android-arm64,android-x64`; `flutter run` takes no such flag and builds the attached device's ABI. `android/app/build.gradle.kts` pins both 64-bit ABIs in `ndk.abiFilters`, which cargokit honours when it builds `host_flutter` — 32-bit devices are dropped instead of pulling a second Rust target.

Android builds `host_flutter` with `--no-default-features --features stems` (`rust_builder/android/build.gradle`) and loads ONNX Runtime at runtime from the `com.microsoft.onnxruntime:onnxruntime-android` AAR (`ort`'s `load-dynamic`). All stem work on Android runs on the CPU EP: NNAPI, XNNPACK, QNN and TVM all measured slower for these models, and desktop WebGPU is slower than CPU for the live model. Desktop offline separation still tries WebGPU first. The desktop-only `stems-webgpu` feature stays off on Android. Models download on first use into the app files dir, like the desktop builds.

Which provider the live producer picked shows up in the app log:

```sh
adb shell run-as top.mixar.app tail -f files/mixar.log | grep "StemgenRT live"
# StemgenRT live: session ready ep=cpu states=13 …
# StemgenRT live: producer is slower than realtime ep=cpu avg_hop_ms=10.97 realtime=0.26
```

A stock build reports `ep=cpu`; a forced provider shows up as that label.

Override for experiments without rebuilding: `adb shell setprop debug.mixar.stems_ep xnnpack` (or `cpu`, `nnapi`, `qnn`), then reload the deck — the property is read when the session is built. Normal playback never uses an accelerator. Measured on a Galaxy device, NNAPI costs 219.6 ms/hop against XNNPACK 10.97 and CPU 11.30 (budget 2.90 ms); numbers and the bench command are in `crates/analyzer-stems/PERF-RUN.md`.

A custom ONNX Runtime build can replace the AAR without code changes:
`MIXAR_EXTRA_JNI_DIR=/path/to/jniDir flutter build apk --debug`, where the
directory is jniLibs-shaped (`arm64-v8a/libonnxruntime.so` plus any EP runtime
libraries). This is how QNN was tested: ONNX Runtime v1.31 built with
`--use_qnn=static_lib --qnn_home <QAIRT>`; on the S23's HTP it ran StemgenRT at
13–71 ms/hop versus ~3 ms on CPU, so QNN is not wired into the cascade. NNAPI
cannot share that build with QNN (duplicate `NodeAttrHelper` at link time).
`MainActivity` sets `ADSP_LIBRARY_PATH` to the native library dir and the
manifest declares `libcdsprpc.so` for QNN's Hexagon backend.

Gradle details worth knowing: file_picker 13 is federated, and its Android module (`android_file_picker`) applies KGP itself whenever `android.builtInKotlin=false`, which this project keeps because AGP 9.x bundles Kotlin 2.2.10, below Flutter's 2.2.20 floor. `android/build.gradle.kts` keeps KGP 2.4.0 (the version declared in `settings.gradle.kts`) on the root classpath so it outranks the 1.8.22 that module declares in its own buildscript. `desktop_drop` stays on 0.8.4 until file_picker 14 moves to `cross_file` 0.4 — 0.9.x, the release that stops Flutter's KGP warning, needs it. `minSdk` is 29 for `libamidi` (midir) and `libaaudio` (cpal), and `rust_builder/cargokit` carries local patches for Gradle 9 (`ExecOperations` instead of `project.exec`), for the NDK/CMake environment native crates like `opusic-sys` require, for `ndk.abiFilters` platform filtering, and for per-platform cargo flags.

After Rust API changes (requires `flutter_rust_bridge_codegen` 2.13.0):

```sh
moon run gui-flutter:generate
```

The main window wraps **MixarTheme** + **ShadTheme** (intentional dual bridge from `main.dart`): header, waveforms, decks/mixer, library. Prefer Mixar shell widgets and `context.theme` tokens; toast errors with `showMixarToast`. Engine, library, and MIDI controller transports are wired over FRB. Display name comes from Rust (`bus_api::APP_DISPLAY_NAME` / `appDisplayName()`). Typography: Outfit / Space Grotesk / Noto Sans Mono via `MixarFonts` (`lib/shell/app_typography.dart`).

## Rust tests

```sh
cargo nextest run --manifest-path crates/Cargo.toml -p host_flutter
```

Uses the `null` backend (no real audio device).
