import 'package:flutter/foundation.dart'
    show TargetPlatform, defaultTargetPlatform, kDebugMode, kIsWeb;
import 'package:gui_flutter/core/utils/desktop_stub.dart'
    if (dart.library.io) 'package:gui_flutter/core/utils/desktop_io.dart'
    as platform;

/// Test override — set `false` in widget tests that don't init
/// `window_manager`.
bool? debugOverrideDesktopWindow;

/// Desktop hosts where `window_manager` applies (not web / mobile).
bool get isDesktopWindow =>
    debugOverrideDesktopWindow ?? (!kIsWeb && platform.ioIsDesktop);

/// Test override — makes a debug Linux run draw the macOS title-bar layout.
///
/// Every test that sets it must restore it to `null`.
bool? debugOverrideMacOSChromePreview;

/// Whether this debug Linux run previews the macOS title-bar layout.
///
/// The macOS build hides AppKit's own buttons and the app draws
/// close/minimise/zoom in the header's leading corner (see
/// `MacOSTitleBarChrome`). This flag renders that same cluster — the same
/// live controls — on Linux, so the layout can be judged without a Mac.
///
/// Off unless switched on for a debug run, either with
/// `--dart-define=MIXAR_PREVIEW_MACOS_CHROME=true` or, because the everyday
/// `npm run app:linux` passes no defines, with
/// `MIXAR_PREVIEW_MACOS_CHROME=true npm run app:linux`. Never in profile or
/// release builds: [kDebugMode] gates the define, the environment and the
/// test override alike.
bool get isMacOSChromePreview =>
    kDebugMode &&
    isDesktopWindow &&
    !kIsWeb &&
    defaultTargetPlatform == TargetPlatform.linux &&
    (debugOverrideMacOSChromePreview ??
        (const bool.fromEnvironment('MIXAR_PREVIEW_MACOS_CHROME') ||
            platform.ioMacOSChromePreview));

/// A macOS desktop window: it hides AppKit's own window buttons and draws
/// close / minimise / zoom in the header instead.
///
/// The single predicate behind both halves of that deal — hiding the native
/// buttons without drawing the app's would leave the window with no controls.
bool get isMacOSHost =>
    isDesktopWindow && !kIsWeb && defaultTargetPlatform == TargetPlatform.macOS;

/// Whether the headers follow the macOS layout: a traffic-light cluster in
/// the leading corner and no app-drawn buttons on the trailing edge.
///
/// True on a Mac (where the cluster *is* the window's controls) and in the
/// Linux preview (which draws and wires the same cluster).
bool get usesMacOSTitleBarLayout => isMacOSHost || isMacOSChromePreview;

/// Leading zone the macOS traffic-light cluster owns, in logical pixels. The
/// cluster is centred in it and the header content starts after it, on every
/// macOS-layout host and fullscreen included, so nothing moves or sits under
/// a button.
const double kMacOSWindowControlsInset = 80;

/// Android, where the app is a plain activity instead of a window
/// (orientation lock, immersive mode, no `window_manager`).
bool get isAndroidHost =>
    !kIsWeb && defaultTargetPlatform == TargetPlatform.android;

/// Hosts where the Rust engine can open an audio output, so decks can load and
/// play: desktop and Android. Web returns `null` from
/// `engineTransportProvider` — there is no engine in the browser build — and
/// the mixer runs as a library shell.
///
/// Android reaches the engine through cpal's AAudio host, which reads the
/// JavaVM and Application context from `ndk-context`; `MainActivity` hands them
/// over before Dart starts (see `initAndroidContext` in `android.rs`).
bool get supportsAudioEngine =>
    debugOverrideDesktopWindow ??
    audioEngineHost(desktop: platform.ioIsDesktop, android: isAndroidHost);

/// The rule behind [supportsAudioEngine], as a predicate over what the platform
/// reports: Android is an engine host without being a desktop window, which is
/// the arm a test has to check on a desktop machine.
bool audioEngineHost({required bool desktop, required bool android}) =>
    !kIsWeb && (desktop || android);

/// Touch-first hosts, where a list row drag must be armed by a long press.
///
/// On these lists a finger pan is how you scroll, and an immediate drag
/// recognizer claims that pan before the scroll view can: a scroll attempt on
/// a row picks the track up instead. Delaying the drag's start until a long
/// press leaves the pan with the list, and a mouse-only host keeps dragging
/// straight off the press.
bool get usesLongPressRowDrag => isAndroidHost;
