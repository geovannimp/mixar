import 'package:gui_flutter/providers/mixer/engine_providers.dart';

/// Records the tracks a deck load asked for, without touching the engine.
class RecordingDeckLoadTransport implements DeckLoadTransport {
  final List<(int deckId, String trackId)> libraryLoads = [];
  final List<(int deckId, String path)> pathLoads = [];

  @override
  Future<void> loadLibraryTrack({
    required int deckId,
    required String trackId,
  }) async {
    libraryLoads.add((deckId, trackId));
  }

  @override
  Future<void> loadPath({required int deckId, required String path}) async {
    pathLoads.add((deckId, path));
  }
}
