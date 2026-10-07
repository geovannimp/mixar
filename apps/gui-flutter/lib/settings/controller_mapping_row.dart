import 'package:flutter/widgets.dart';
import 'package:gui_flutter/library/library_list_chrome.dart'
    show MetaPill, kMetaPillGap;
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
    // Metadata as the library track row's chips: id, device id, version, and
    // an accent "Attached" state. The pill's default fill is the row surface
    // (`card`), which is invisible on this settings card, so use the page
    // background fill + border (matching the library's status pill).
    MetaPill chip(String text, {Color? textColor, FontWeight? fontWeight}) =>
        MetaPill(
          text: text,
          textColor: textColor,
          fontWeight: fontWeight ?? FontWeight.w500,
          backgroundColor: theme.colors.background,
          borderColor: theme.colors.border,
        );
    final metaPills = <Widget>[
      chip(mapping.id),
      chip(mapping.deviceId),
      if (version != null) chip('v$version'),
      if (attached)
        chip(
          'Attached',
          textColor: theme.colors.primary,
          fontWeight: FontWeight.w600,
        ),
    ];

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
                if (metaPills.isNotEmpty)
                  Wrap(
                    spacing: kMetaPillGap,
                    runSpacing: kMetaPillGap,
                    children: metaPills,
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
