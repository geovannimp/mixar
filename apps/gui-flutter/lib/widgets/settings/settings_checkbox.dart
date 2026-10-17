import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/mixar_checkbox.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// Left-aligned checkbox row for settings fields and settings-style forms.
///
/// Card/section headers keep a switch instead — see `SettingsHeaderSwitch`.
class SettingsCheckbox extends StatelessWidget {
  const new({
    required this.label,
    required this.value,
    required this.onChanged,
    super.key,
  });

  final String label;
  final bool value;
  final ValueChanged<bool> onChanged;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return Align(
      alignment: Alignment.centerLeft,
      child: MixarCheckbox(
        value: value,
        onChanged: onChanged,
        label: Text(
          label,
          style: theme.typography.body.sm.copyWith(
            color: theme.colors.foreground,
            fontWeight: FontWeight.w600,
          ),
        ),
      ),
    );
  }
}
