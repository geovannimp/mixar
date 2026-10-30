import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/misc.dart' show ProviderListenable;
import 'package:gui_flutter/core/utils/track_drag.dart';
import 'package:gui_flutter/providers/library/drawer_providers.dart';
import 'package:gui_flutter/providers/library/focused_load.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/providers/shell/app_notice.dart';

/// Compatible with `Ref.read` / `WidgetRef.read` / `ProviderContainer.read`.
typedef DeckLoadRead = T Function<T>(ProviderListenable<T> provider);

/// MIDI / controller load: the active list's focused row → deck.
///
/// Watches [pendingFocusedDeckLoadProvider] so library event bootstrap never
/// imports these helpers (breaks library ↔ engine cycle).
///
/// Serializes loads and keeps only the latest pending request (last-wins) so a
/// second MIDI load while one is in flight cannot interleave `applyTrackDrop`.
final focusedDeckLoadBootstrapProvider = Provider<void>((ref) {
  Future<void>? inFlight;
  PendingFocusedDeckLoadRequest? queued;

  Future<void> pump() async {
    while (queued != null) {
      final req = queued!;
      queued = null;
      try {
        final current = ref.read(pendingFocusedDeckLoadProvider);
        // Last-wins: skip a dequeued request already superseded in the
        // provider (a newer listen updated state + [queued]).
        if (current?.seq != req.seq) {
          continue;
        }
        ref.read(pendingFocusedDeckLoadProvider.notifier).clear();
        await loadFocusedRowToDeck(ref, req.deckId);
      } catch (e) {
        ref.read(appNoticeProvider.notifier).setError('$e');
      }
    }
  }

  void ensurePump() {
    if (inFlight != null) {
      return;
    }
    inFlight = pump().whenComplete(() {
      inFlight = null;
      // Request may land after the loop drains but before [inFlight] clears.
      if (queued != null) {
        ensurePump();
      }
    });
  }

  ref.listen<PendingFocusedDeckLoadRequest?>(pendingFocusedDeckLoadProvider, (
    prev,
    next,
  ) {
    if (next == null) {
      return;
    }
    queued = next;
    ensurePump();
  });
});

/// MIDI / controller load: the active list's focused row → deck.
///
/// The focused payload is published by the active track list, so this works for
/// the library list and the history list alike without knowing which is on
/// screen.
Future<void> loadFocusedRowToDeck(Ref ref, int deckId) async {
  final focused = ref.read(focusedTrackPayloadProvider);
  final payload = focused.owner == null ? null : focused.payload;
  if (payload == null) {
    return;
  }
  await loadPayloadOntoDeck(ref.read, deckId, payload);
}

/// Shared deck-load path (UI drop/keyboard + MIDI).
///
/// Uses [deckLoadGenerationProvider] so a superseded attempt does not call the
/// engine or overwrite chrome after a newer load wins.
Future<void> loadPayloadOntoDeck(
  DeckLoadRead read,
  int deckId,
  TrackDragPayload payload,
) async {
  // Resolved before the first await, and before the drawer below closes: the
  // widget that asked for this load lives *inside* the drawer (the track list,
  // the deck picker popover), so popping it disposes the caller's `WidgetRef`
  // and every `read` after that throws. Nothing past this point may call read.
  final generations = read(deckLoadGenerationProvider.notifier);
  final loading = read(deckLoadInFlightProvider.notifier);
  final engineUi = read(engineUiProvider.notifier);
  final notices = read(appNoticeProvider.notifier);
  final transportFuture = read(deckLoadTransportProvider.future);

  // Every load path lands here — a library drag, the row's Load to deck menu,
  // the keyboard Enter / Shift+Enter intents, the deck picker, and the
  // controller's focused-row load — so the drawer is dismissed for all of
  // them. Skipped mid-drag: the drag source lives inside the drawer and its
  // own end closes the panel.
  if (!read(libraryTrackDragActiveProvider)) {
    read(libraryDrawerOpenProvider.notifier).set(false);
  }

  final generation = generations.begin(deckId);
  loading.set(deckId, true);
  try {
    final transport = await transportFuture;
    if (transport == null) {
      return;
    }
    if (!generations.isCurrent(deckId, generation)) {
      return;
    }
    await applyTrackDrop(
      deckId: deckId,
      payload: payload,
      loadLibraryTrack: (id, trackId) =>
          transport.loadLibraryTrack(deckId: id, trackId: trackId),
      loadPath: (id, path) => transport.loadPath(deckId: id, path: path),
    );
    if (!generations.isCurrent(deckId, generation)) {
      return;
    }
    engineUi
      ..setDeckTrackPath(deckId, payload.path)
      ..setDeckTrackId(deckId, payload.trackId);
  } catch (e) {
    // Superseded loads stay silent, same contract as the isCurrent guards.
    if (!generations.isCurrent(deckId, generation)) {
      return;
    }
    notices.setError('$e');
  } finally {
    try {
      loading.set(deckId, false);
    } on Object {
      // The container was torn down while the engine load was in flight.
    }
  }
}

Future<void> loadPayloadToDeck(
  WidgetRef ref,
  int deckId,
  TrackDragPayload payload,
) => loadPayloadOntoDeck(ref.read, deckId, payload);
