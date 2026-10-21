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
