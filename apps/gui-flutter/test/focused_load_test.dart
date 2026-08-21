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

  test('focusedLoadPayload prefers library track id', () {
    final tracks = [_track('t1'), _track('t2')];
    expect(
      focusedLoadPayload(tracks, 1, inLibrary: (_) => true),
      TrackDragPayload(
        source: TrackDragSource.library,
        trackId: 't2',
        path: '/tmp/t2.wav',
        title: 't2',
      ),
    );
  });

  test('focusedLoadPayload uses path for filesystem rows', () {
    final tracks = [
      LibraryTrackSummary(
        id: '/tmp/a.wav',
        displayName: 'a.wav',
        path: '/tmp/a.wav',
      ),
    ];
    expect(
      focusedLoadPayload(tracks, 0, inLibrary: (_) => false),
      TrackDragPayload(
        source: TrackDragSource.filesystem,
        path: '/tmp/a.wav',
        title: 'a.wav',
      ),
    );
  });

  test('focusedLoadPayload is null off the table', () {
    expect(focusedLoadPayload(const [], 0, inLibrary: (_) => true), isNull);
    expect(
      focusedLoadPayload([_track('t1')], 1, inLibrary: (_) => true),
      isNull,
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
