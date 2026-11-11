import 'package:flutter/gestures.dart' show kLongPressTimeout;
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/track_drag.dart';

/// How long a touch host holds a row before its drag is armed: the framework's
/// long-press timeout, plus a frame's slack for the timer to land.
///
/// Widget tests report `TargetPlatform.android`, so a row drag there waits for
/// a long press — the same hold a finger makes on a phone, and the reason a
/// scroll never picks a track up. See `usesLongPressRowDrag`.
final rowDragArm = kLongPressTimeout + const Duration(milliseconds: 50);

/// Presses at [from] and holds until the row's drag is armed, leaving the
/// gesture down so the test can move it onto a drop target.
Future<TestGesture> startRowDrag(WidgetTester tester, Offset from) async {
  final gesture = await tester.startGesture(from);
  await tester.pump(rowDragArm);
  return gesture;
}

/// Any library row drag source, whatever the host: `LongPressDraggable` is a
/// `Draggable` subclass, so `find.byType(Draggable<…>)` would miss the touch
/// host's variant.
Finder rowDragSources() =>
    find.byWidgetPredicate((widget) => widget is Draggable<TrackDragPayload>);
