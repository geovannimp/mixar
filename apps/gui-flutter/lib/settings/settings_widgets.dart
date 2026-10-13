import 'package:flutter/widgets.dart';
import 'package:gui_flutter/settings/settings_field.dart';
import 'package:gui_flutter/shell/m_card.dart';
import 'package:gui_flutter/shell/mixar_select.dart';
import 'package:gui_flutter/shell/mixar_switch.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';

class SettingsToggle extends StatelessWidget {
  const new({
    required this.label,
    required this.value,
    required this.onChanged,
    super.key,
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

/// Titled settings card: required title + subtitle above optional [child].
///
/// Pass [trailing] for header actions (e.g. Update All, enable toggles). Omit
/// [child] when the card is header-only (toggle off, no fields to show).
class SettingsSectionCard extends StatelessWidget {
  const new({
    required this.title,
    required this.subtitle,
    super.key,
    this.trailing,
    this.child,
  });

  final String title;
  final String subtitle;
  final Widget? trailing;
  final Widget? child;

  @override
  Widget build(BuildContext context) {
    final header = SettingsSectionHeader(title: title, description: subtitle);
    return MCard(
      child: Padding(
        padding: const .fromLTRB(16, 16, 16, 16),
        child: Column(
          crossAxisAlignment: .start,
          spacing: 16,
          children: [
            if (trailing == null)
              header
            else
              Row(
                crossAxisAlignment: .start,
                spacing: 12,
                children: [
                  Expanded(child: header),
                  trailing!,
                ],
              ),
            if (child != null) child!,
          ],
        ),
      ),
    );
  }
}

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
