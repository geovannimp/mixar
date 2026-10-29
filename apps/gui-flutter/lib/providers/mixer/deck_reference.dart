import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/utils/key_format.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/core/utils/tempo_format.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/providers/shell/app_notice.dart';
import 'package:gui_flutter/src/rust/api/library.dart';

/// How long a changed deck reference waits before reaching the library.
///
/// Riding the tempo fader must not re-evaluate a smart playlist per frame, so
/// the push trails the last change instead of following it.
const kDeckReferenceDebounce = Duration(milliseconds: 500);

/// One deck's sounding values, as deck-relative smart-playlist rules read them.
///
/// Equal by value so the bridge can skip pushes that change nothing.
@immutable
class DeckReferenceValue {
  const new({required this.deckId, required this.playing, this.bpm, this.key});

  /// Zero-based deck index (deck A is 0).
  final int deckId;
  final bool playing;

  /// Sounding BPM (tempo fader applied); null when the deck holds no track.
  final double? bpm;

  /// Sounding key (Key Shift and, without key lock, the tempo ratio applied).
  final String? key;

  LibraryDeckReference toFrb() => LibraryDeckReference(
    deckId: deckId,
    playing: playing,
    bpm: bpm,
    key: key,
  );

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is DeckReferenceValue &&
          deckId == other.deckId &&
          playing == other.playing &&
          bpm == other.bpm &&
          key == other.key;

  @override
  int get hashCode => Object.hash(deckId, playing, bpm, key);
}

/// Every deck's sounding values, one entry per configured deck.
final deckReferencesProvider = Provider<List<DeckReferenceValue>>((ref) {
  final settings = ref
      .watch(appSettingsProvider)
      .maybeWhen(data: (value) => value, orElse: defaultAppSettings);
  return [
    for (var deck = 0; deck < settings.numDecks; deck++)
      _deckReference(ref, deck),
  ];
});

DeckReferenceValue _deckReference(Ref ref, int deckId) {
  final playing = ref.watch(deckPlayingProvider(deckId));
  if (ref.watch(deckTrackIdProvider(deckId)) == null) {
    return DeckReferenceValue(deckId: deckId, playing: playing);
  }
  final track = ref.watch(deckLibraryTrackProvider(deckId));
  final speed = ref.watch(deckSpeedProvider(deckId));
  final tempoRange = ref.watch(deckTempoRangeProvider(deckId));
  // Same mapping the UI uses for the sounding tempo: the fader norm counter-
  // rotates against the range, so a hand-rolled `norm - 0.5` would invert it.
  final ratio = normToSpeedRatio(speed, tempoRange);
  final keyLock = ref.watch(deckKeyLockProvider(deckId));
  final semitones =
      ref.watch(deckKeyShiftProvider(deckId)) +
      (keyLock ? 0 : 12 * math.log(ratio) / math.ln2);
  final key = track?.key;
  return DeckReferenceValue(
    deckId: deckId,
    playing: playing,
    bpm: effectiveBpm(track?.bpm, speed, tempoRange),
    key: key == null || key.trim().isEmpty
        ? null
        : transposeKey(key, semitones.round()) ?? key,
  );
}

/// Pushes [deckReferencesProvider] to the library, debounced, and re-evaluates
/// the open smart playlist when the reference actually changes.
///
/// Mounted once by the shell. The library only does the percent and harmonic
/// math; this provider owns everything the engine knows (fader, key shift).
final deckReferenceBridgeProvider = Provider<void>((ref) {
  Timer? pending;
  List<DeckReferenceValue>? applied;

  /// The value being pushed right now, so a recompute during the round trip
  /// does not queue a second identical push.
  List<DeckReferenceValue>? inFlight;

  Future<void> push(List<DeckReferenceValue> decks) async {
    inFlight = decks;
    try {
      final transport = await ref.read(libraryTransportProvider.future);
      await transport.setDeckReference(
        decks: [for (final deck in decks) deck.toFrb()],
      );
    } on Object catch (error) {
      if (identical(inFlight, decks)) {
        inFlight = null;
      }
      if (!ref.mounted) {
        return;
      }
      // Not recorded as applied: the next change (or a retry) pushes again.
      ref.read(appNoticeProvider.notifier).setError('$error');
      return;
    }
    if (!ref.mounted) {
      return;
    }
    // Only the newest push owns the bridge: a slower earlier one must not
    // record itself as the library's state, or the next identical reference
    // would push a duplicate.
    final owned = identical(inFlight, decks);
    if (!owned) {
      return;
    }
    inFlight = null;
    applied = decks;
    if (ref.read(activeCollectionIsSmartProvider)) {
      ref.invalidate(collectionTracksProvider);
    }
  }

  void schedule(List<DeckReferenceValue> decks) {
    // The deck count comes from settings; the fallback used while they load
    // would brief the library with the wrong reference.
    if (ref.read(appSettingsProvider).isLoading) {
      return;
    }
    // Cancel first: a fader that returns to the applied value must not leave an
    // older intermediate push armed.
    pending?.cancel();
    pending = null;
    if (listEquals(decks, applied) || listEquals(decks, inFlight)) {
      return;
    }
    if (applied == null && inFlight == null) {
      // Nothing is known to the library yet: seed it before the first edit.
      unawaited(push(decks));
      return;
    }
    pending = Timer(kDeckReferenceDebounce, () => unawaited(push(decks)));
  }

  ref
    ..onDispose(() => pending?.cancel())
    ..listen(
      deckReferencesProvider,
      (_, decks) => schedule(decks),
      fireImmediately: true,
    );
});
