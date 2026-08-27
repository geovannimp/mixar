import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';

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
            child: FSwitch(value: value, onChange: onChanged),
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
    return FCard(
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
    return FSelect<T>.rich(
      control: .lifted(
        value: value,
        onChange: (next) {
          if (next != null) {
            onChanged(next);
          }
        },
      ),
      enabled: enabled,
      format: labelBuilder,
      contentOverlayLocation: OverlayChildLocation.rootOverlay,
      children: [
        for (final option in options)
          FSelectItem.item(
            title: Text(labelBuilder(option)),
            subtitle: subtitleBuilder == null
                ? null
                : Text(subtitleBuilder!(option)),
            value: option,
          ),
      ],
    );
  }
}
