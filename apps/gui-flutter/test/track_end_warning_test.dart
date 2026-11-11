import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/providers/mixer/track_end_warning.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';

import 'support/bus_events.dart';

void main() {
  ProviderContainer containerWithWarning({
    required int seconds,
    bool enabled = true,
    double? bpm,
    double speedRatio = 1,
  }) {
    final container = ProviderContainer(
      overrides: [
        appSettingsProvider.overrideWith(
          (ref) async => copyAppSettings(
            defaultAppSettings(),
            trackEndWarning: enabled,
            trackEndWarningSecs: seconds,
          ),
        ),
        deckBpmProvider.overrideWith((ref, deckId) => bpm),
        deckSpeedRatioProvider.overrideWith((ref, deckId) => speedRatio),
      ],
    );
    addTearDown(container.dispose);
    return container;
  }

  Future<void> loadDeck(
    ProviderContainer container, {
    required bool playing,
    int durationMs = 60000,
  }) async {
    await container.read(appSettingsProvider.future);
    container
        .read(engineUiProvider.notifier)
        .apply(
          deckUpdatedEvent(
            update: deckUpdated(
              trackId: 'track',
              playing: playing,
              durationMs: durationMs,
              positionMs: 0,
            ),
          ),
        );
  }

  void seek(ProviderContainer container, int positionMs) {
    container
        .read(engineUiProvider.notifier)
        .apply(deckPositionEvent(positionMs: positionMs));
  }

  test('turns on inside the warning window while playing', () async {
    final container = containerWithWarning(seconds: 30);
    await loadDeck(container, playing: true);

    seek(container, 20000);
    expect(container.read(deckTrackEndWarningProvider(0)), isFalse);
    expect(container.read(trackEndWarningSecsProvider), 30);

    seek(container, 31000);
    expect(container.read(deckTrackEndWarningProvider(0)), isTrue);
  });

  test('stays off while the deck is paused', () async {
    final container = containerWithWarning(seconds: 30);
    await loadDeck(container, playing: false);

    seek(container, 59000);

    expect(container.read(deckTrackEndWarningProvider(0)), isFalse);
  });

  test('the settings toggle disables the warning', () async {
    final container = containerWithWarning(seconds: 30, enabled: false);
    await loadDeck(container, playing: true);

    seek(container, 59999);

    expect(container.read(deckTrackEndWarningProvider(0)), isFalse);
  });

  test('a bigger window starts the warning earlier', () async {
    final container = containerWithWarning(seconds: 60);
    await loadDeck(container, playing: true);

    seek(container, 1000);

    expect(container.read(deckTrackEndWarningProvider(0)), isTrue);
  });

  test('a loaded but stopped position past the end does not warn', () async {
    final container = containerWithWarning(seconds: 30);
    await loadDeck(container, playing: true);

    seek(container, 60000);

    expect(container.read(deckTrackEndWarningProvider(0)), isFalse);
  });

  test('a playhead beyond the duration does not warn', () async {
    final container = containerWithWarning(seconds: 30);
    await loadDeck(container, playing: true);

    // Past the end the remaining time is negative; a blink there would never
    // stop on its own.
    seek(container, 61000);

    expect(container.read(deckTrackEndWarningProvider(0)), isFalse);
  });

  test('the blink runs one period every half bar of the track tempo', () {
    Duration period(double? bpm, {double ratio = 1}) => containerWithWarning(
      seconds: 30,
      bpm: bpm,
      speedRatio: ratio,
    ).read(deckEndWarningBlinkPeriodProvider(0));

    // 2 beats x 60 000 ms / bpm.
    expect(period(120), const Duration(milliseconds: 1000));
    expect(period(128), const Duration(milliseconds: 938));
    expect(period(174), const Duration(milliseconds: 690));
  });

  test('the blink follows the pitch fader, not the analyzed tempo', () {
    Duration period({required double ratio}) => containerWithWarning(
      seconds: 30,
      bpm: 120,
      speedRatio: ratio,
    ).read(deckEndWarningBlinkPeriodProvider(0));

    expect(period(ratio: 1), const Duration(milliseconds: 1000));
    // +6 % on a 120 BPM track sounds like 127.2 BPM.
    expect(period(ratio: 1.06), const Duration(milliseconds: 943));
    expect(period(ratio: 0.94), const Duration(milliseconds: 1064));
  });

  test('the blink period stays inside its usable bounds', () {
    Duration period(double bpm) => containerWithWarning(
      seconds: 30,
      bpm: bpm,
    ).read(deckEndWarningBlinkPeriodProvider(0));

    expect(period(400), kEndWarningMinBlinkPeriod);
    expect(period(5), kEndWarningMaxBlinkPeriod);
    expect(
      containerWithWarning(seconds: 30)
          .read(deckEndWarningBlinkPeriodProvider(0)),
      kEndWarningFallbackBlinkPeriod,
      reason: 'no analyzed tempo falls back to the fixed cue',
    );
  });
}
