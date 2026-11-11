import 'package:flutter/foundation.dart'
    show TargetPlatform, defaultTargetPlatform, kIsWeb;
import 'package:gui_flutter/core/utils/desktop_stub.dart'
    if (dart.library.io) 'package:gui_flutter/core/utils/desktop_io.dart'
    as platform;

/// Test override — set `false` in widget tests that don't init
/// `window_manager`.
bool? debugOverrideDesktopWindow;

/// Desktop hosts where `window_manager` applies (not web / mobile).
bool get isDesktopWindow =>
    debugOverrideDesktopWindow ?? (!kIsWeb && platform.ioIsDesktop);

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
