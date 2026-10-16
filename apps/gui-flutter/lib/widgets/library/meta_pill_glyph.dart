import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// Muted pill glyph, sized for the metadata pills.
class MetaPillGlyph extends StatelessWidget {
  const new(this.icon, {super.key});

  final IconData icon;

  @override
  Widget build(BuildContext context) =>
      Icon(icon, size: 12, color: context.theme.colors.mutedForeground);
}
