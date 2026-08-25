import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/shell/controller_providers.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_field.dart';
import 'package:gui_flutter/src/rust/api/controller.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';

class SettingsControllersPanel extends ConsumerStatefulWidget {
  const SettingsControllersPanel({
    required this.draft,
    required this.onChanged,
    super.key,
  });

  final AppSettings draft;
  final ValueChanged<AppSettings> onChanged;

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

  void _setTrusted(String deviceId, bool trusted) {
    final next = List<String>.from(widget.draft.trustedControllerDeviceIds);
    if (trusted) {
      if (!next.contains(deviceId)) {
        next.add(deviceId);
      }
    } else {
      next.remove(deviceId);
    }
    widget.onChanged(
      copyAppSettings(widget.draft, trustedControllerDeviceIds: next),
    );
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final mappings = ref.watch(controllerMappingsProvider);
    final devices = ref.watch(controllerDevicesProvider);
    final attachedId = ref.watch(attachedMappingIdProvider);
    final transport = ref.watch(controllerTransportProvider).value;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const SettingsSectionHeader(
          title: 'Controllers',
          description:
              'MIDI mappings live in app data. Seed copies shipped maps when missing; Update overwrites from the app bundle. Trust device auto-enables on connect after Save.',
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
                      trusted: widget.draft.trustedControllerDeviceIds.contains(
                        mapping.deviceId,
                      ),
                      busy: _busy || transport == null,
                      onToggleAttach: (enabled) => _run(
                        () => enabled
                            ? transport!.enableMapping(mappingId: mapping.id)
                            : transport!.disableMapping(mappingId: mapping.id),
                      ),
                      onToggleTrust: (trusted) =>
                          _setTrusted(mapping.deviceId, trusted),
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
        const SizedBox(height: 16),
        SettingsField(
          label: 'MIDI ports',
          child: devices.when(
            data: (rows) {
              if (rows.isEmpty) {
                return Text(
                  'No MIDI ports detected.',
                  style: theme.typography.body.sm.copyWith(
                    color: theme.colors.mutedForeground,
                  ),
                );
              }
              return Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  for (final device in rows)
                    Padding(
                      padding: const EdgeInsets.symmetric(vertical: 2),
                      child: Text(
                        [
                          device.direction.name,
                          device.portName,
                          if (device.matchedMappingId != null)
                            '→ ${device.matchedMappingId}',
                        ].join(' '),
                        style: theme.typography.body.xs.copyWith(
                          color: theme.colors.mutedForeground,
                        ),
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
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
  required bool trusted,
  required bool busy,
  required ValueChanged<bool> onToggleAttach,
  required ValueChanged<bool> onToggleTrust,
  required VoidCallback onUpdate,
}) {
  final name = [
    mapping.vendorName,
    mapping.productName,
  ].where((s) => s.isNotEmpty).join(' ');
  return FItem(
    title: Text(name),
    subtitle: Text(
      '${mapping.id} · ${mapping.deviceId}${attached ? ' · attached' : ''}',
    ),
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
        Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.end,
          children: [
            Row(
              mainAxisSize: MainAxisSize.min,
              spacing: 6,
              children: [
                Text('Trust', style: TextStyle(fontSize: 11)),
                SizedBox(
                  height: 23,
                  child: FittedBox(
                    child: FSwitch(
                      value: trusted,
                      enabled: !busy,
                      semanticsLabel: 'Trust device $name',
                      onChange: busy ? null : onToggleTrust,
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
                Text('Attach', style: TextStyle(fontSize: 11)),
                SizedBox(
                  height: 23,
                  child: FittedBox(
                    child: FSwitch(
                      value: attached,
                      enabled: !busy,
                      semanticsLabel: 'Enable $name',
                      onChange: busy ? null : onToggleAttach,
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
