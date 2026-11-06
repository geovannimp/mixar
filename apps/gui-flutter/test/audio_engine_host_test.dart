import 'package:flutter/foundation.dart'
    show TargetPlatform, debugDefaultTargetPlatformOverride;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/desktop.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';

/// The engine gate decides whether decks can load at all: the library row's
/// `Draggable`, the deck's `DragTarget` and the row menu's Load to deck all
/// follow `engineRunningProvider`, which only turns on once
/// `engineTransportProvider` starts. It used to be desktop-only, so Android had
/// no way to put a track on a deck.
void main() {
  tearDown(() {
    debugDefaultTargetPlatformOverride = null;
    debugOverrideDesktopWindow = null;
  });

  test('Android is an engine host without being a desktop window', () {
    // The arm that matters here: a phone is not a window, and used to fall out
    // of the gate entirely.
    expect(audioEngineHost(desktop: false, android: true), isTrue);
  });

  test('desktop hosts keep the engine', () {
    expect(audioEngineHost(desktop: true, android: false), isTrue);
  });

  test('a mobile host outside Android stays library-only', () {
    expect(audioEngineHost(desktop: false, android: false), isFalse);
  });

  test('isAndroidHost follows the platform', () {
    debugDefaultTargetPlatformOverride = TargetPlatform.android;
    expect(isAndroidHost, isTrue);

    debugDefaultTargetPlatformOverride = TargetPlatform.linux;
    expect(isAndroidHost, isFalse);
  });

  test('the test override still skips native audio', () {
    // Widget tests set this so no provider opens a device.
    debugOverrideDesktopWindow = false;

    expect(supportsAudioEngine, isFalse);
  });

  test('the gate keeps the transport null on a host without an engine', () {
    debugOverrideDesktopWindow = false;

    final container = ProviderContainer();
    addTearDown(container.dispose);

    // Resolves instead of starting an engine: `engineTransportProvider` returns
    // null before it reaches any transport, so nothing touches native audio.
    expect(container.read(engineTransportProvider.future), completion(isNull));
  });
}
