import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/mixer/track_drag.dart';
import 'package:gui_flutter/src/rust/api/library.dart';

int navigateIndex(int current, int count, int delta) {
  if (count <= 0) {
    return 0;
  }
  final next = current + delta;
  if (next < 0) {
    return 0;
  }
  if (next > count - 1) {
    return count - 1;
  }
  return next;
}

TrackDragPayload payloadFromListTrack(
  LibraryTrackSummary track, {
  required bool inLibrary,
}) {
  final title = (track.title?.isNotEmpty ?? false)
      ? track.title!
      : track.displayName;
  return TrackDragPayload(
    source: inLibrary ? TrackDragSource.library : TrackDragSource.filesystem,
    trackId: inLibrary ? track.id : null,
    path: track.path,
    title: trackDisplayTitle(title: title, path: track.path),
  );
}

/// One-shot MIDI / controller "load focused row" request.
///
/// Library event bootstrap publishes here; focused deck-load bootstrap
/// consumes it so library providers never import deck load helpers.
@immutable
class PendingFocusedDeckLoadRequest {
  const new({required this.deckId, required this.seq});

  final int deckId;
  final int seq;
}

class PendingFocusedDeckLoad extends Notifier<PendingFocusedDeckLoadRequest?> {
  var _seq = 0;

  @override
  PendingFocusedDeckLoadRequest? build() => null;

  void request(int deckId) {
    _seq += 1;
    state = PendingFocusedDeckLoadRequest(deckId: deckId, seq: _seq);
  }

  void clear() => state = null;
}

final pendingFocusedDeckLoadProvider =
    NotifierProvider<PendingFocusedDeckLoad, PendingFocusedDeckLoadRequest?>(
      PendingFocusedDeckLoad.new,
    );
