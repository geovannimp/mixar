import 'package:flutter/widgets.dart';
import 'package:gui_flutter/shell/app_button.dart';
import 'package:gui_flutter/shell/mixar_switch.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:gui_flutter/src/rust/api/controller.dart';

class ControllerMappingRow extends StatelessWidget {
  const new({
    required this.mapping,
    required this.attached,
    required this.trusted,
    required this.attachBusy,
    required this.trustBusy,
    required this.onToggleAttach,
    required this.onToggleTrust,
    required this.onUpdate,
    super.key,
  });

  final ControllerMappingInfo mapping;
  final bool attached;
  final bool trusted;
  final bool attachBusy;
  final bool trustBusy;
  final ValueChanged<bool> onToggleAttach;
  final ValueChanged<bool> onToggleTrust;
  final VoidCallback onUpdate;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final name = [
      mapping.vendorName,
      mapping.productName,
    ].where((s) => s.isNotEmpty).join(' ');
    final version = mapping.version;
    final meta = [
      mapping.id,
      mapping.deviceId,
      if (version != null) 'v$version',
      if (attached) 'attached',
    ].join(' · ');

    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 6),
      child: Row(
        spacing: 12,
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              spacing: 2,
              children: [
                Text(
                  name,
                  style: theme.typography.body.sm.copyWith(
                    fontWeight: FontWeight.w600,
                  ),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
                Text(
                  meta,
                  style: theme.typography.body.xs.copyWith(
                    color: theme.colors.mutedForeground,
                  ),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
              ],
            ),
          ),
          if (mapping.updateAvailable)
            Text(
              'Update available',
              style: theme.typography.body.xs.copyWith(
                color: theme.colors.primary,
                fontWeight: FontWeight.w600,
              ),
            ),
          AppButton(
            variant: .outline,
            size: .sm,
            mainAxisSize: .min,
            onPress: attachBusy ? null : onUpdate,
            child: const Text('Update'),
          ),
          Row(
            mainAxisSize: MainAxisSize.min,
            spacing: 12,
            children: [
              _LabeledSwitch(
                label: 'Trust',
                value: trusted,
                enabled: !trustBusy,
                semanticsLabel: 'Trust device $name',
                onChanged: onToggleTrust,
              ),
              _LabeledSwitch(
                label: 'Attach',
                value: attached,
                enabled: !attachBusy,
                semanticsLabel: 'Enable $name',
                onChanged: onToggleAttach,
              ),
            ],
          ),
        ],
      ),
    );
  }
}

/// Compact `label + switch` pair for the mapping row's trailing controls.
class _LabeledSwitch extends StatelessWidget {
  const new({
    required this.label,
    required this.value,
    required this.enabled,
    required this.semanticsLabel,
    required this.onChanged,
  });

  final String label;
  final bool value;
  final bool enabled;
  final String semanticsLabel;
  final ValueChanged<bool> onChanged;

  @override
  Widget build(BuildContext context) {
    return Row(
      mainAxisSize: MainAxisSize.min,
      spacing: 6,
      children: [
        Text(label, style: const TextStyle(fontSize: 11)),
        SizedBox(
          height: 23,
          child: FittedBox(
            child: MixarSwitch(
              value: value,
              enabled: enabled,
              semanticsLabel: semanticsLabel,
              onChanged: enabled ? onChanged : null,
            ),
          ),
        ),
      ],
    );
  }
}
