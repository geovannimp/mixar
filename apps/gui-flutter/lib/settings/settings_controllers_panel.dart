import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/shell/controller_providers.dart';
import 'package:gui_flutter/settings/settings_field.dart';
import 'package:gui_flutter/src/rust/api/controller.dart';

class SettingsControllersPanel extends ConsumerStatefulWidget {
  const SettingsControllersPanel({super.key});

  @override
  ConsumerState<SettingsControllersPanel> createState() =>
      _SettingsControllersPanelState();
}

class _SettingsControllersPanelState
    extends ConsumerState<SettingsControllersPanel> {
  var _busy = false;

  Future<void> _run(Future<void> Function() action) async {
    setState(() => _busy = true);
    try {
      await action();
    } catch (e) {
      if (mounted) {
        showFToast(context: context, variant: .destructive, title: Text('$e'));
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final mappings = ref.watch(controllerMappingsProvider);
    final attachedId = ref.watch(attachedMappingIdProvider);
    final transport = ref.watch(controllerTransportProvider).value;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const SettingsSectionHeader(
          title: 'Controllers',
          description:
              'MIDI mappings live in app data. Seed copies shipped maps when missing; Update overwrites from the app bundle.',
        ),
        const SizedBox(height: 16),
        SettingsField(
          label: 'Mappings',
          trailing: FButton(
            variant: .outline,
            size: .sm,
            mainAxisSize: .min,
            onPress: transport == null || _busy
                ? null
                : () => _run(transport.updateAllMappings),
            child: const Text('Update All'),
          ),
          child: mappings.when(
            data: (rows) {
              if (rows.isEmpty) {
                return Text(
                  'No mappings in app data yet.',
                  style: theme.typography.body.sm.copyWith(
                    color: theme.colors.mutedForeground,
                  ),
                );
              }
              return FItemGroup(
                divider: .indented,
                physics: const NeverScrollableScrollPhysics(),
                children: [
                  for (final mapping in rows)
                    _mappingItem(
                      mapping: mapping,
                      attached: mapping.id == attachedId,
                      busy: _busy || transport == null,
                      onToggle: (enabled) => _run(
                        () => enabled
                            ? transport!.enableMapping(mappingId: mapping.id)
                            : transport!.disableMapping(mappingId: mapping.id),
                      ),
                      onUpdate: () => _run(
                        () => transport!.updateMapping(mappingId: mapping.id),
                      ),
                    ),
                ],
              );
            },
            loading: () => Text(
              'Loading…',
              style: theme.typography.body.sm.copyWith(
                color: theme.colors.mutedForeground,
              ),
            ),
            error: (e, _) => Text(
              '$e',
              style: theme.typography.body.sm.copyWith(
                color: theme.colors.destructive,
              ),
            ),
          ),
        ),
      ],
    );
  }
}

FItem _mappingItem({
  required ControllerMappingInfo mapping,
  required bool attached,
  required bool busy,
  required ValueChanged<bool> onToggle,
  required VoidCallback onUpdate,
}) {
  final name = [
    mapping.vendorName,
    mapping.productName,
  ].where((s) => s.isNotEmpty).join(' ');
  return FItem(
    title: Text(name),
    subtitle: Text('${mapping.id} · ${mapping.deviceId}'),
    details: attached ? const Text('attached') : null,
    suffix: Row(
      mainAxisSize: MainAxisSize.min,
      spacing: 8,
      children: [
        FButton(
          variant: .outline,
          size: .sm,
          mainAxisSize: .min,
          onPress: busy ? null : onUpdate,
          child: const Text('Update'),
        ),
        SizedBox(
          height: 23,
          child: FittedBox(
            child: FSwitch(
              value: attached,
              enabled: !busy,
              semanticsLabel: 'Enable $name',
              onChange: busy ? null : onToggle,
            ),
          ),
        ),
      ],
    ),
  );
}
