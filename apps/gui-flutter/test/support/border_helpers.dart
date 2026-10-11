import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

/// The [Border] of the decorated box matched by [finder].
///
/// Supports [DecoratedBox.decoration] and [Container] decoration /
/// [Container.foregroundDecoration] (tab bars paint the hairline in
/// foreground so the selected chip cannot cover it).
///
/// Fails readably (rather than crashing on a cast) when the box's decoration is
/// not a bordered [BoxDecoration], so a shape change surfaces as an assertion.
Border borderOf(WidgetTester tester, Finder finder) {
  final widget = tester.widget(finder);
  if (widget is DecoratedBox) {
    final decoration = widget.decoration;
    expect(decoration, isA<BoxDecoration>());
    final border = (decoration as BoxDecoration).border;
    expect(border, isA<Border>());
    return border! as Border;
  }
  if (widget is Container) {
    for (final decoration in [
      widget.foregroundDecoration,
      widget.decoration,
    ]) {
      if (decoration is BoxDecoration && decoration.border is Border) {
        return decoration.border! as Border;
      }
    }
  }
  fail(
    'Expected DecoratedBox or Container with a Border, '
    'found ${widget.runtimeType}',
  );
}
