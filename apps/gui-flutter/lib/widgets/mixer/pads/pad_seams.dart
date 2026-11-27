import 'package:flutter/widgets.dart';

/// Seam policy a grid hands to the pads it lays out.
///
/// The grid cannot know what sits outside its box, so it can tell the pads in
/// its bottom row to skip their bottom hairline: the surface below (the deck's
/// transport divider, or a bordered panel's frame) already draws that line.
class PadSeams extends InheritedWidget {
  const new({required this.bottom, super.key, required super.child});

  /// Whether the pad paints its bottom hairline.
  final bool bottom;

  static bool bottomOf(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<PadSeams>()?.bottom ?? true;

  @override
  bool updateShouldNotify(PadSeams oldWidget) => bottom != oldWidget.bottom;
}
