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
npm run flutter:dev
# or: moon run gui-flutter:dev-linux
```

Desktop uses [`window_manager`](https://pub.dev/packages/window_manager) with `TitleBarStyle.hidden` and in-app min/max/close controls (drag empty header regions to move; double-click to maximize).

### Android

The engine and the controller transport are desktop-only (`isDesktopWindow` in `lib/core/utils/desktop.dart`), so Android boots the library shell without audio playback and without a mapped controller session. First-run setup still probes MIDI once (`detectControllers`), because the setup step is built from what the OS reports.

Android runs landscape-only and full-screen: the manifest pins `screenOrientation="sensorLandscape"` and `main.dart` switches to `SystemUiMode.immersiveSticky` before the first frame. Android 16+ ignores the orientation lock on large screens (≥600 dp), where the windowing system wins.

App data lands in the platform files dir (`/data/user/<id>/top.mixar.app/files/` → `library.db`, `settings.json`, `mixar.log`). Rust cannot derive it — Android app processes have no `HOME`/`XDG_DATA_HOME`, which used to resolve to `/` and fail with `I/O error: Read-only file system` — so `main.dart` passes `path_provider`'s application support directory to `setAppSupportDir` before any transport opens; `init_app` skips log attachment on Android and `setAppSupportDir` attaches it.

MIDI needs one more piece of glue the Flutter embedding does not provide: `ndk-context`. midir's Android backend and cpal's AAudio host read the JavaVM and an Application context from it, and nothing initializes it here — a Flutter app has no `android_activity` glue crate. `MainActivity.onCreate` therefore loads `libhost_flutter.so` and calls `initAndroidContext(applicationContext)` before `super.onCreate` starts Dart; the JNI export lives in `crates/host-flutter/src/android.rs` (the package's `_` is `_1` in the mangled symbol, so renaming either side renames the function). Without it the first MIDI probe panics with "android context was not initialized", and the Dart future awaiting it never completes: first-run setup stays on its spinner with nothing in `mixar.log` but that one line.

One-time host setup (SDK location can also come from `ANDROID_HOME`):

```sh
flutter config --android-sdk "$HOME/Android/sdk"
mise exec -- sdkmanager "ndk;28.2.13676358"   # flutter.ndkVersion for Flutter 3.47.7
rustup target add aarch64-linux-android x86_64-linux-android
```

Then, with a device or emulator up (`flutter emulators --launch Pixel_7_Pro_API_36`, `adb devices`):

```sh
npm run flutter:android               # moon run gui-flutter:dev-android
moon run gui-flutter:build-android    # debug APK in build/app/outputs/flutter-apk/
```

`build-android` passes `--target-platform android-arm64,android-x64`; `flutter run` takes no such flag and builds the attached device's ABI. `android/app/build.gradle.kts` pins both 64-bit ABIs in `ndk.abiFilters`, which cargokit honours when it builds `host_flutter` — 32-bit devices are dropped instead of pulling a second Rust target.

Android builds `host_flutter` with `--no-default-features` (`rust_builder/android/build.gradle`), which turns the `stems` feature off: ONNX Runtime publishes no Android x86_64 binary, and the engine (the only consumer of stems) is not started on mobile anyway. Stem jobs report "unavailable on this platform" instead of linking ORT.

Gradle details worth knowing: `android/build.gradle.kts` applies KGP to `:file_picker` itself, because file_picker 11 skips KGP whenever AGP is 9+ (assuming built-in Kotlin) while this project keeps `android.builtInKotlin=false` — AGP 9.x bundles Kotlin 2.2.10, below Flutter's 2.2.20 floor, so KGP 2.4.0 (declared in `settings.gradle.kts`) is what compiles. That block also pins that module's Kotlin `jvmTarget` to 17, matching the `compileOptions` file_picker sets. `minSdk` is 29 for `libamidi` (midir) and `libaaudio` (cpal), and `rust_builder/cargokit` carries local patches for Gradle 9 (`ExecOperations` instead of `project.exec`), for the NDK/CMake environment native crates like `opusic-sys` require, for `ndk.abiFilters` platform filtering, and for per-platform cargo flags.

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
