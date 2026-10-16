import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// Uppercase sidebar section label (Tauri `LibraryPaneHeader`).
class LibraryPaneLabel extends StatelessWidget {
  const new(this.text, {super.key});

  final String text;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return Text(
      text.toUpperCase(),
      style: theme.typography.body.xs.copyWith(
        color: theme.colors.mutedForeground,
        fontWeight: FontWeight.w600,
        letterSpacing: 1.4,
      ),
    );
  }
}
