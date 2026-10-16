import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// Filled surface behind a list pane's rows.
///
/// Flush and borderless: the list fills its pane edge to edge, so the row fill
/// is all this needs — no radius or border insets it from the panel edges.
class LibraryListSurface extends StatelessWidget {
  const new({required this.theme, required this.child, super.key});

  final MixarThemeData theme;
  final Widget child;

  @override
  Widget build(BuildContext context) =>
      ColoredBox(color: theme.colors.card, child: child);
}
