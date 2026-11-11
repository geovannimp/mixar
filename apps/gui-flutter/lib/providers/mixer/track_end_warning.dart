import 'dart:math' as math;

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';

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

/// Read one setting, falling back while the settings are still loading.
///
/// Both warning fields go through this so their load-time default can never
/// drift apart.
T _setting<T>(Ref ref, T Function(AppSettings settings) read, T fallback) => ref
    .watch(appSettingsProvider)
    .maybeWhen(data: read, orElse: () => fallback);

/// Warning window before a track ends, in seconds.
final trackEndWarningSecsProvider = Provider<int>((ref) {
  return _setting(
    ref,
    (settings) => settings.trackEndWarningSecs,
    kDefaultTrackEndWarningSecs,
  );
});

/// True while the deck plays inside its end-of-track warning window.
///
/// Re-evaluated on every 30 Hz position tick, but it yields a plain `bool`, so
/// Riverpod's value equality keeps the watchers still until the flag actually
/// flips — the waveform and the track card only rebuild when the window opens
/// or closes.
final deckTrackEndWarningProvider = Provider.family<bool, int>((ref, deckId) {
  final enabled = _setting(
    ref,
    (settings) => settings.trackEndWarning,
    kDefaultTrackEndWarningEnabled,
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
