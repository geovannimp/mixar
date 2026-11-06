import 'package:flutter/foundation.dart'
    show TargetPlatform, defaultTargetPlatform, kIsWeb;
import 'package:gui_flutter/core/utils/desktop_stub.dart'
    if (dart.library.io) 'package:gui_flutter/core/utils/desktop_io.dart'
    as platform;

/// Test override — set `false` in widget tests that don't init [window_manager].
bool? debugOverrideDesktopWindow;

/// Desktop hosts where [window_manager] applies (not web / mobile).
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
    (!kIsWeb && (platform.ioIsDesktop || isAndroidHost));
