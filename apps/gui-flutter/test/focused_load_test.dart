import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/library/focused_load.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/mixer/track_drag.dart';
import 'package:gui_flutter/src/rust/api/library.dart';

LibraryTrackSummary _track(String id, {String? path}) {
  return LibraryTrackSummary(
    id: id,
    displayName: id,
    path: path ?? '/tmp/$id.wav',
  );
}

void main() {
  test('navigateIndex clamps to visible rows', () {
    expect(navigateIndex(0, 0, 1), 0);
    expect(navigateIndex(0, 5, 2), 2);
    expect(navigateIndex(0, 5, -1), 0);
    expect(navigateIndex(4, 5, 3), 4);
  });

  test('payloadFromListTrack prefers the library track id', () {
    expect(
      payloadFromListTrack(_track('t1'), inLibrary: true),
      const TrackDragPayload(
        source: TrackDragSource.library,
        trackId: 't1',
        path: '/tmp/t1.wav',
        title: 't1',
      ),
    );
  });

  test('payloadFromListTrack uses the path for filesystem rows', () {
    const track = LibraryTrackSummary(
      id: '/tmp/a.wav',
      displayName: 'a.wav',
      path: '/tmp/a.wav',
    );
    expect(
      payloadFromListTrack(track, inLibrary: false),
      const TrackDragPayload(
        source: TrackDragSource.filesystem,
        path: '/tmp/a.wav',
        title: 'a.wav',
      ),
    );
  });

  test('FocusedTrackRowIndex navigates then clamps to row count', () {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    final focus = container.read(focusedTrackRowIndexProvider.notifier);
    focus.setCount(5);
    focus.navigate(2);
    expect(container.read(focusedTrackRowIndexProvider), 2);
    focus.navigate(9);
    expect(container.read(focusedTrackRowIndexProvider), 4);
    focus.setCount(2);
    expect(container.read(focusedTrackRowIndexProvider), 1);
  });
}
