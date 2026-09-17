import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/shell/m_card.dart';
import 'package:gui_flutter/shell/mixar_select.dart';
import 'package:gui_flutter/shell/mixar_switch.dart';

class SettingsToggle extends StatelessWidget {
  const SettingsToggle({
    super.key,
    required this.label,
    required this.value,
    required this.onChanged,
    this.labelStyle,
  });

  final String label;
  final bool value;
  final ValueChanged<bool> onChanged;
  final TextStyle? labelStyle;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return Row(
      children: [
        Expanded(
          child: Text(
            label,
            style:
                labelStyle ??
                theme.typography.body.sm.copyWith(
                  color: theme.colors.foreground,
                  fontWeight: FontWeight.w600,
                ),
          ),
        ),
        SizedBox(
          height: 23,
          child: FittedBox(
            child: MixarSwitch(value: value, onChanged: onChanged),
          ),
        ),
      ],
    );
  }
}

class SettingsPanel extends StatelessWidget {
  const SettingsPanel({super.key, required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    return MCard(
      child: Padding(padding: const .fromLTRB(16, 16, 16, 16), child: child),
    );
  }
}

class SettingsSelect<T> extends StatelessWidget {
  const SettingsSelect({
    super.key,
    required this.value,
    required this.options,
    required this.labelBuilder,
    required this.onChanged,
    this.subtitleBuilder,
    this.enabled = true,
  });

  final T value;
  final List<T> options;
  final String Function(T value) labelBuilder;
  final String Function(T value)? subtitleBuilder;
  final ValueChanged<T> onChanged;
  final bool enabled;

  @override
  Widget build(BuildContext context) {
    return MixarSelect<T>(
      value: value,
      options: options,
      labelBuilder: labelBuilder,
      subtitleBuilder: subtitleBuilder,
      onChanged: onChanged,
      enabled: enabled,
    );
  }
}
