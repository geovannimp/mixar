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

TrackDragPayload payloadFromTableTrack(
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

TrackDragPayload? focusedLoadPayload(
  List<LibraryTrackSummary> tracks,
  int index, {
  required bool Function(LibraryTrackSummary track) inLibrary,
}) {
  if (index < 0 || index >= tracks.length) {
    return null;
  }
  final track = tracks[index];
  return payloadFromTableTrack(track, inLibrary: inLibrary(track));
}

/// Grid row for [tableIndex] in provider order, even when the table is sorted.
int? visualRowIndexForFocusedTrack(
  List<String?> visualTrackIds,
  List<String> tableTrackIds,
  int tableIndex,
) {
  if (tableIndex < 0 || tableIndex >= tableTrackIds.length) {
    return null;
  }
  final i = visualTrackIds.indexOf(tableTrackIds[tableIndex]);
  return i < 0 ? null : i;
}
