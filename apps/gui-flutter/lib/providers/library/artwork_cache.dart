import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/src/rust/api/library.dart';

/// In-memory artwork keyed by track id
/// (`getTrack` only — lists stay artwork-free).
class ArtworkCache extends Notifier<Map<String, Uint8List?>> {
  /// shortcut: session map of cover blobs; cap + drop non-visible first.
  /// Eviction among eligible keys is insertion-order FIFO (LinkedHashMap),
  /// not LRU. Upgrade path: true LRU / disk thumbnail cache once covers
  /// live in library.db.
  static const _maxEntries = 300;

  final Set<String> _inFlight = {};

  /// Latest visible window — eviction hint only. Not used to cancel loads:
  /// concurrent [ensureLoaded] replaces this set, and claimed in-flight ids
  /// must still complete (doc: stores completions that scrolled away).
  Set<String> _wanted = {};

  @override
  Map<String, Uint8List?> build() => {};

  /// Fetch missing ids for [ids]; still stores completions that scrolled away.
  Future<void> ensureLoaded(List<String> ids) async {
    _wanted = ids.toSet();
    // Claim in-flight synchronously so concurrent unawaited ensureLoaded
    // calls cannot both schedule getTrack for the same id.
    final missing = <String>[
      for (final id in ids)
        if (!state.containsKey(id) && _inFlight.add(id)) id,
    ];
    if (missing.isEmpty) {
      return;
    }
    // shortcut: batch pool — at most 3 concurrent getTrack calls
    for (var i = 0; i < missing.length; i += 3) {
      final batch = missing.skip(i).take(3);
      await Future.wait(batch.map(_loadOne));
    }
  }

  Future<Uint8List?> _loadArtwork(String id) async {
    final LibraryTransport transport;
    try {
      transport = await ref.read(libraryTransportProvider.future);
    } on Object {
      // The transport itself failed to open (no bridge, unreadable DB). The
      // library pane already reports that once; repeating it for every visible
      // row would drown the log. Artwork stays empty, the null sentinel is
      // stored by the caller.
      return null;
    }
    final track = await transport.getTrack(trackId: id);
    return track?.artwork;
  }

  Future<void> _loadOne(String id) async {
    // [id] was already claimed in [ensureLoaded] via [_inFlight.add].
    // Do not gate on [_wanted] here: a later ensureLoaded replaces that set
    // and would drop claimed ids. _inFlight already coalesces duplicates.
    try {
      final bytes = await _loadArtwork(id);
      _store(id, bytes);
    } on Object catch (error, stack) {
      // Prefetch is unawaited: catch Exception and Error so one bad id
      // cannot abort later batches. Store null sentinel so scroll/deck
      // ensureLoaded stops retrying (#461). Recovery is eviction from the
      // 300-entry map — not per-frame refetch. Report both Exception
      // (FRB/I/O) and Error so failures are not silent in logs.
      _store(id, null);
      FlutterError.reportError(
        FlutterErrorDetails(
          exception: error,
          stack: stack,
          library: 'artwork_cache',
          context: ErrorDescription('loading artwork for $id'),
        ),
      );
    } finally {
      _inFlight.remove(id);
    }
  }

  void _store(String id, Uint8List? bytes) {
    final next = Map<String, Uint8List?>.of(state)..[id] = bytes;
    if (next.length > _maxEntries) {
      final drop = next.length - _maxEntries;
      // Prefer dropping non-wanted (latest visible window). If every key is
      // still wanted, fall back to oldest wanted (insertion FIFO) so the
      // hard cap never overshoots — accepted trade-off vs placeholder flash.
      final nonWanted = [
        for (final k in next.keys)
          if (k != id && !_wanted.contains(k)) k,
      ];
      final wantedFallback = [
        for (final k in next.keys)
          if (k != id && _wanted.contains(k)) k,
      ];
      final victims = [...nonWanted, ...wantedFallback].take(drop).toSet();
      next.removeWhere((k, _) => victims.contains(k));
    }
    state = next;
  }
}

final artworkCacheProvider =
    NotifierProvider<ArtworkCache, Map<String, Uint8List?>>(ArtworkCache.new);
