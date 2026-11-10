import 'package:flutter/widgets.dart';

/// One solid, slightly rounded rectangle inside a preview glyph.
///
/// Preview painters compose these instead of drawing raw rectangles so every
/// settings glyph shares the same corner treatment.
class SettingsPreviewBlock extends StatelessWidget {
  const new({required this.color, this.radius = 1, super.key});

  final Color color;
  final double radius;

  @override
  Widget build(BuildContext context) => DecoratedBox(
    decoration: BoxDecoration(
      color: color,
      borderRadius: BorderRadius.circular(radius),
    ),
  );
}
