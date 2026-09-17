import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/shell/app_button.dart';
import 'package:gui_flutter/src/rust/api/controller.dart';

class ControllerMappingRow extends StatelessWidget {
  const ControllerMappingRow({
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
    final meta =
        '${mapping.id} · ${mapping.deviceId}${attached ? ' · attached' : ''}';

    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 6),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.center,
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
          AppButton(
            variant: .outline,
            size: .sm,
            mainAxisSize: .min,
            onPress: attachBusy ? null : onUpdate,
            child: const Text('Update'),
          ),
          Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.end,
            children: [
              Row(
                mainAxisSize: MainAxisSize.min,
                spacing: 6,
                children: [
                  const Text('Trust', style: TextStyle(fontSize: 11)),
                  SizedBox(
                    height: 23,
                    child: FittedBox(
                      child: FSwitch(
                        value: trusted,
                        enabled: !trustBusy,
                        semanticsLabel: 'Trust device $name',
                        onChange: trustBusy ? null : onToggleTrust,
                      ),
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 4),
              Row(
                mainAxisSize: MainAxisSize.min,
                spacing: 6,
                children: [
                  const Text('Attach', style: TextStyle(fontSize: 11)),
                  SizedBox(
                    height: 23,
                    child: FittedBox(
                      child: FSwitch(
                        value: attached,
                        enabled: !attachBusy,
                        semanticsLabel: 'Enable $name',
                        onChange: attachBusy ? null : onToggleAttach,
                      ),
                    ),
                  ),
                ],
              ),
            ],
          ),
        ],
      ),
    );
  }
}
