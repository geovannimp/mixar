import 'dart:typed_data';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/library/providers.dart';

/// In-memory artwork keyed by track id (`getTrack` only — lists stay artwork-free).
class ArtworkCache extends Notifier<Map<String, Uint8List?>> {
  final Set<String> _inFlight = {};
  Set<String> _wanted = {};

  @override
  Map<String, Uint8List?> build() => {};

  /// Fetch missing ids for [ids]; ignores completions when [ids] no longer wanted.
  Future<void> ensureLoaded(WidgetRef ref, List<String> ids) async {
    _wanted = ids.toSet();
    final missing = [
      for (final id in ids)
        if (!state.containsKey(id) && !_inFlight.contains(id)) id,
    ];
    if (missing.isEmpty) {
      return;
    }
    // ponytail: batch pool — at most 3 concurrent getTrack calls
    for (var i = 0; i < missing.length; i += 3) {
      final batch = missing.skip(i).take(3);
      await Future.wait(batch.map((id) => _loadOne(ref, id)));
    }
  }

  Future<void> _loadOne(WidgetRef ref, String id) async {
    if (!_wanted.contains(id)) {
      return;
    }
    _inFlight.add(id);
    try {
      final transport = await ref.read(libraryTransportProvider.future);
      final track = await transport.getTrack(trackId: id);
      if (!_wanted.contains(id)) {
        return;
      }
      state = {...state, id: track?.artwork};
    } catch (_) {
      if (!_wanted.contains(id)) {
        return;
      }
      state = {...state, id: null};
    } finally {
      _inFlight.remove(id);
    }
  }
}

final artworkCacheProvider =
    NotifierProvider<ArtworkCache, Map<String, Uint8List?>>(ArtworkCache.new);
