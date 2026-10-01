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
