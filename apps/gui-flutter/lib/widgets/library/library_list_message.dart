import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// Centred message for a list pane's empty / loading / error states.
class LibraryListMessage extends StatelessWidget {
  const new(this.text, {required this.color, super.key});

  final String text;
  final Color color;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Text(
        text,
        style: context.theme.typography.body.sm.copyWith(color: color),
      ),
    );
  }
}
