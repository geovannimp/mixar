import 'dart:async';

import 'package:file_picker/file_picker.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/library/focused_load.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/mixer/track_drag.dart';

/// MIDI / controller load: the active list's focused row → deck.
///
/// Watches [pendingFocusedDeckLoadProvider] so library event bootstrap never
/// imports these helpers (breaks library ↔ engine cycle).
final focusedDeckLoadBootstrapProvider = Provider<void>((ref) {
  ref.listen<PendingFocusedDeckLoadRequest?>(pendingFocusedDeckLoadProvider, (
    prev,
    next,
  ) {
    if (next == null) {
      return;
    }
    ref.read(pendingFocusedDeckLoadProvider.notifier).clear();
    unawaited(loadFocusedRowToDeck(ref, next.deckId));
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

  final loading = ref.read(deckLoadInFlightProvider.notifier);
  loading.set(deckId, true);
  try {
    final engine = await ref.read(engineTransportProvider.future);
    if (engine == null) {
      return;
    }
    await applyTrackDrop(
      deckId: deckId,
      payload: payload,
      loadLibraryTrack: (id, trackId) =>
          engine.loadLibraryTrack(deckId: id, trackId: trackId),
      loadPath: (id, path) => engine.loadPath(deckId: id, path: path),
    );
    ref.read(engineUiProvider.notifier).setDeckTrackPath(deckId, payload.path);
    ref.read(engineUiProvider.notifier).setDeckTrackId(deckId, payload.trackId);
  } finally {
    loading.set(deckId, false);
  }
}

Future<void> loadPayloadToDeck(
  WidgetRef ref,
  int deckId,
  TrackDragPayload payload,
) async {
  final loading = ref.read(deckLoadInFlightProvider.notifier);
  loading.set(deckId, true);
  try {
    final engine = await ref.read(engineTransportProvider.future);
    if (engine == null) {
      return;
    }
    await applyTrackDrop(
      deckId: deckId,
      payload: payload,
      loadLibraryTrack: (id, trackId) =>
          engine.loadLibraryTrack(deckId: id, trackId: trackId),
      loadPath: (id, path) => engine.loadPath(deckId: id, path: path),
    );
    ref.read(engineUiProvider.notifier).setDeckTrackPath(deckId, payload.path);
    ref.read(engineUiProvider.notifier).setDeckTrackId(deckId, payload.trackId);
  } finally {
    loading.set(deckId, false);
  }
}

Future<void> pickTrackForDeck(WidgetRef ref, int deckId) async {
  final result = await FilePicker.pickFiles(
    type: FileType.custom,
    allowedExtensions: supportedAudioExtensions,
  );
  final path = result?.files.single.path;
  if (path == null || path.isEmpty) {
    return;
  }
  await loadPayloadToDeck(ref, deckId, payloadFromOsPath(path));
}
