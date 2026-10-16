import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/settings/settings_field.dart';
import 'package:gui_flutter/widgets/shell/mixar_select.dart';

class SettingsSelect<T> extends StatelessWidget {
  const new({
    required this.value,
    required this.options,
    required this.labelBuilder,
    required this.onChanged,
    super.key,
    this.subtitleBuilder,
    this.enabled = true,
    this.dialogTitle,
  });

  final T value;
  final List<T> options;
  final String Function(T value) labelBuilder;
  final String Function(T value)? subtitleBuilder;
  final ValueChanged<T> onChanged;
  final bool enabled;

  /// Title for the mobile dialog picker (usually the [SettingsField] label).
  final String? dialogTitle;

  @override
  Widget build(BuildContext context) {
    return MixarSelect<T>(
      value: value,
      options: options,
      labelBuilder: labelBuilder,
      subtitleBuilder: subtitleBuilder,
      onChanged: onChanged,
      enabled: enabled,
      dialogTitle: dialogTitle,
    );
  }
}
