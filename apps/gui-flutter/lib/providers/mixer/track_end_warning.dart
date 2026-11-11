import 'dart:math' as math;

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';

/// Beats per blink: half a 4/4 bar, so the cue lands on beats 1 and 3.
const kEndWarningBlinkBeats = 2;

/// Floor for the blink period. The fade shrinks with the period, so this only
/// has to keep a gap between blinks on an absurdly fast track (240 BPM bets a
/// 500 ms half bar).
const kEndWarningMinBlinkPeriod = Duration(milliseconds: 500);

/// Ceiling, for a very slow track: the cue must still recur.
const kEndWarningMaxBlinkPeriod = Duration(seconds: 10);

/// Only reached if a track has no analyzed tempo at all — analysis is
/// required, so this is a guard, not a normal path.
const kEndWarningFallbackBlinkPeriod = Duration(seconds: 2);

/// Warning window before a track ends, in seconds.
final trackEndWarningSecsProvider = Provider<int>((ref) {
  return ref
      .watch(appSettingsProvider)
      .maybeWhen(
        data: (settings) => settings.trackEndWarningSecs,
        orElse: () => kDefaultTrackEndWarningSecs,
      );
});

/// True while the deck plays inside its end-of-track warning window.
///
/// Only flips on the crossing, so the 30 Hz position ticks never rebuild the
/// widgets watching it — the waveform and the track card just start blinking.
final deckTrackEndWarningProvider = Provider.family<bool, int>((ref, deckId) {
  final enabled = ref
      .watch(appSettingsProvider)
      .maybeWhen(
        data: (settings) => settings.trackEndWarning,
        orElse: () => kDefaultTrackEndWarningEnabled,
      );
  if (!enabled) {
    return false;
  }
  final warningSecs = ref.watch(trackEndWarningSecsProvider);
  final durationMs = ref.watch(deckDurationMsProvider(deckId)) ?? 0;
  if (durationMs <= 0) {
    return false;
  }
  // A paused deck is not about to run out; blinking it would only nag.
  if (!ref.watch(deckPlayingProvider(deckId))) {
    return false;
  }
  final remainingMs = durationMs - ref.watch(deckPositionMsProvider(deckId));
  return remainingMs > 0 && remainingMs <= warningSecs * 1000;
});

/// How often the warning blinks: half a bar of the deck's **sounding** tempo,
/// so the blink follows the pitch fader instead of a wall clock.
final deckEndWarningBlinkPeriodProvider = Provider.family<Duration, int>((
  ref,
  deckId,
) {
  final bpm = ref.watch(deckBpmProvider(deckId));
  if (bpm == null || !bpm.isFinite || bpm <= 0) {
    return kEndWarningFallbackBlinkPeriod;
  }
  final soundingBpm = bpm * ref.watch(deckSpeedRatioProvider(deckId));
  if (!soundingBpm.isFinite || soundingBpm <= 0) {
    return kEndWarningFallbackBlinkPeriod;
  }
  final periodMs = (kEndWarningBlinkBeats * 60000 / soundingBpm).round();
  return Duration(
    milliseconds: math.max(
      kEndWarningMinBlinkPeriod.inMilliseconds,
      math.min(periodMs, kEndWarningMaxBlinkPeriod.inMilliseconds),
    ),
  );
});
