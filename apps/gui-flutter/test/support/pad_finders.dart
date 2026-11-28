import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

/// The pad's chrome decoration (the outer [DecoratedBox] inside [pad]).
BoxDecoration padDecorationOf(WidgetTester tester, Finder pad) {
  return tester
          .widget<DecoratedBox>(
            find.descendant(of: pad, matching: find.byType(DecoratedBox)).first,
          )
          .decoration
      as BoxDecoration;
}

/// Parks a mouse pointer outside every pad, so a test can move it onto the pad
/// it means to hover. The pointer is removed when the test ends.
///
/// The default sits outside the widget tree entirely: any in-bounds point can
/// land on a pad in a small fixture.
Future<TestGesture> attachMousePointer(
  WidgetTester tester, {
  Offset at = const Offset(-1, -1),
}) async {
  final pointer = await tester.createGesture(kind: PointerDeviceKind.mouse);
  addTearDown(pointer.removePointer);
  await pointer.addPointer(location: at);
  return pointer;
}
