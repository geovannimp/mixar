import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/settings/controller_mapping_row.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_field.dart';
import 'package:gui_flutter/settings/settings_widgets.dart';
import 'package:gui_flutter/shell/app_button.dart';
import 'package:gui_flutter/shell/controller_providers.dart';
import 'package:gui_flutter/shell/m_divider.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:gui_flutter/shell/mixar_toast.dart';
import 'package:gui_flutter/src/rust/api/controller.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';

class SettingsControllersPanel extends ConsumerStatefulWidget {
  const new({required this.draft, required this.onChanged, super.key});

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
        showMixarToast(
          context: context,
          variant: MixarToastVariant.destructive,
          title: Text('$e'),
        );
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
    final l10n = AppLocalizations.of(context)!;
    final mappings = ref.watch(controllerMappingsProvider);
    final devices = ref.watch(controllerDevicesProvider);
    final attachedIds = ref.watch(attachedMappingIdsProvider);
    final transport = ref.watch(controllerTransportProvider).value;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      spacing: 16,
      children: [
        SettingsSectionHeader(
          title: l10n.settingsSectionControllers,
          description: l10n.settingsControllersDescription,
        ),
        SettingsPanel(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            spacing: 12,
            children: [
              Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                spacing: 12,
                children: [
                  Expanded(
                    child: SettingsSectionHeader(
                      title: l10n.settingsControllersMappingsTitle,
                      description: l10n.settingsControllersMappingsDescription,
                    ),
                  ),
                  AppButton(
                    variant: .outline,
                    size: .sm,
                    mainAxisSize: .min,
                    onPress: transport == null || _busy
                        ? null
                        : () => _run(() async {
                            await transport.updateAllMappings();
                            if (mounted) {
                              ref.invalidate(controllerMappingsProvider);
                            }
                          }),
                    child: Text(l10n.settingsControllersUpdateAll),
                  ),
                ],
              ),
              mappings.when(
                data: (rows) {
                  if (rows.isEmpty) {
                    return Text(
                      l10n.settingsControllersNoMappings,
                      style: theme.typography.body.sm.copyWith(
                        color: theme.colors.mutedForeground,
                      ),
                    );
                  }
                  return Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      for (var i = 0; i < rows.length; i++) ...[
                        if (i > 0)
                          const MDivider(
                            padding: EdgeInsets.symmetric(vertical: 4),
                          ),
                        ControllerMappingRow(
                          mapping: rows[i],
                          attached: attachedIds.contains(rows[i].id),
                          trusted: widget.draft.trustedControllerDeviceIds
                              .contains(rows[i].deviceId),
                          attachBusy: _busy || transport == null,
                          trustBusy: _busy,
                          onToggleAttach: (enabled) => _run(
                            () => enabled
                                ? transport!.enableMapping(
                                    mappingId: rows[i].id,
                                  )
                                : transport!.disableMapping(
                                    mappingId: rows[i].id,
                                  ),
                          ),
                          onToggleTrust: (trusted) =>
                              _setTrusted(rows[i].deviceId, trusted),
                          onUpdate: () => _run(() async {
                            await transport!.updateMapping(
                              mappingId: rows[i].id,
                            );
                            if (mounted) {
                              ref.invalidate(controllerMappingsProvider);
                            }
                          }),
                        ),
                      ],
                    ],
                  );
                },
                loading: () => Text(
                  l10n.commonLoading,
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
            ],
          ),
        ),
        SettingsPanel(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            spacing: 12,
            children: [
              SettingsSectionHeader(
                title: l10n.settingsControllersMidiPortsTitle,
                description: l10n.settingsControllersMidiPortsDescription,
              ),
              devices.when(
                data: (rows) {
                  if (rows.isEmpty) {
                    return Text(
                      l10n.settingsControllersNoMidiPorts,
                      style: theme.typography.body.sm.copyWith(
                        color: theme.colors.mutedForeground,
                      ),
                    );
                  }
                  return Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      for (var i = 0; i < rows.length; i++) ...[
                        if (i > 0)
                          const MDivider(
                            padding: EdgeInsets.symmetric(vertical: 4),
                          ),
                        _MidiPortRow(device: rows[i]),
                      ],
                    ],
                  );
                },
                loading: () => Text(
                  l10n.commonLoading,
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
            ],
          ),
        ),
      ],
    );
  }
}

/// One detected MIDI port: direction chip, port name, and mapping match.
class _MidiPortRow extends StatelessWidget {
  const new({required this.device});

  final ControllerDeviceInfo device;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final l10n = AppLocalizations.of(context)!;
    final mapping = device.matchedMappingId;
    return Row(
      spacing: 10,
      children: [
        _DirectionBadge(direction: device.direction),
        Expanded(
          child: Text(
            device.portName,
            style: theme.typography.body.sm.copyWith(
              fontWeight: FontWeight.w600,
            ),
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
        ),
        ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 180),
          child: Text(
            mapping == null
                ? l10n.settingsControllersNoMapping
                : l10n.settingsControllersMappingArrow(mapping),
            textAlign: TextAlign.right,
            style: theme.typography.body.xs.copyWith(
              color: mapping == null
                  ? theme.colors.mutedForeground
                  : theme.colors.primary,
            ),
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
        ),
      ],
    );
  }
}

/// Input/output chip for a MIDI port row (green input, teal output).
class _DirectionBadge extends StatelessWidget {
  const new({required this.direction});

  final ControllerDeviceDirection direction;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final l10n = AppLocalizations.of(context)!;
    final input = direction == ControllerDeviceDirection.input;
    final color = input ? theme.colors.primary : theme.colors.accent;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.12),
        border: Border.all(color: color.withValues(alpha: 0.35)),
        borderRadius: theme.style.borderRadius.xs,
      ),
      child: SizedBox(
        width: 44,
        height: 20,
        child: Center(
          child: Text(
            input
                ? l10n.settingsControllersDirectionIn
                : l10n.settingsControllersDirectionOut,
            style: theme.typography.body.xs.copyWith(
              color: color,
              fontWeight: FontWeight.w700,
              letterSpacing: 0.6,
            ),
          ),
        ),
      ),
    );
  }
}
