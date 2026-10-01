import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

/// The [Border] of the decorated box matched by [finder].
///
/// Fails readably (rather than crashing on a cast) when the box's decoration is
/// not a bordered [BoxDecoration], so a shape change surfaces as an assertion.
Border borderOf(WidgetTester tester, Finder finder) {
  final box = tester.widget<DecoratedBox>(finder);
  final decoration = box.decoration;
  expect(decoration, isA<BoxDecoration>());
  final border = (decoration as BoxDecoration).border;
  expect(border, isA<Border>());
  return border! as Border;
}
