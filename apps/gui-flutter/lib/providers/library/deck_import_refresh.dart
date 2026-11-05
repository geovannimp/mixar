import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/providers/shell/app_notice.dart';

/// Re-publish cues and loops for every track currently loaded on a deck.
///
/// Importers write straight into the library database, so a deck that loaded
/// its track before the import keeps the cue list it cached at load time until
/// it reloads. The library worker already re-publishes both lists on
/// `refreshTrack` (for any source, streams included), so an import re-runs it
/// for the loaded tracks only.
///
/// The import has already committed when this runs, so a refresh failure is
/// reported as itself and never as an import failure, and one deck failing must
/// not stop the remaining decks from refreshing.
Future<void> refreshLoadedDeckTracks(WidgetRef ref) async {
  final decks = ref.read(appSettingsProvider).value?.numDecks ?? 2;
  // Snapshot first: a deck can be unloaded while an earlier refresh awaits.
  final trackIds = {
    for (var deck = 0; deck < decks; deck++)
      if (ref.read(deckTrackIdProvider(deck)) case final id? when id.isNotEmpty)
        id,
  };
  final transport = await ref.read(libraryTransportProvider.future);
  for (final trackId in trackIds) {
    try {
      await transport.refreshTrack(trackId: trackId);
    } catch (e) {
      ref.read(appNoticeProvider.notifier).setError('$e');
    }
  }
}
