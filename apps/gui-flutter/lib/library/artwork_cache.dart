import 'dart:typed_data';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/library/providers.dart';

/// In-memory artwork keyed by track id (`getTrack` only — lists stay artwork-free).
class ArtworkCache extends Notifier<Map<String, Uint8List?>> {
  /// ponytail: session map of cover blobs; cap + drop non-visible first.
  /// Upgrade path: true LRU / disk thumbnail cache once covers live in library.db.
  static const _maxEntries = 300;

  final Set<String> _inFlight = {};
  Set<String> _wanted = {};

  @override
  Map<String, Uint8List?> build() => {};

  /// Fetch missing ids for [ids]; still stores completions that scrolled away.
  Future<void> ensureLoaded(List<String> ids) async {
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
      await Future.wait(batch.map(_loadOne));
    }
  }

  Future<void> _loadOne(String id) async {
    if (!_wanted.contains(id)) {
      return;
    }
    _inFlight.add(id);
    try {
      final transport = await ref.read(libraryTransportProvider.future);
      final track = await transport.getTrack(trackId: id);
      _store(id, track?.artwork);
    } catch (_) {
      _store(id, null);
    } finally {
      _inFlight.remove(id);
    }
  }

  void _store(String id, Uint8List? bytes) {
    final next = Map<String, Uint8List?>.of(state)..[id] = bytes;
    if (next.length > _maxEntries) {
      final drop = next.length - _maxEntries;
      final victims = [
        for (final k in next.keys)
          if (k != id && !_wanted.contains(k)) k,
      ].take(drop);
      for (final k in victims) {
        next.remove(k);
      }
    }
    state = next;
  }
}

final artworkCacheProvider =
    NotifierProvider<ArtworkCache, Map<String, Uint8List?>>(ArtworkCache.new);
