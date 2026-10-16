import 'dart:typed_data';

import 'package:flutter/widgets.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/src/rust/api/engine.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_field.dart';
import 'package:gui_flutter/widgets/settings/settings_select.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

class SettingsBusRouteFields extends StatelessWidget {
  const new({
    required this.route,
    required this.devices,
    required this.devicesLoading,
    required this.onChanged,
  });

  final BusRouteSettings route;
  final List<OutputDevice> devices;
  final bool devicesLoading;
  final ValueChanged<BusRouteSettings> onChanged;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final l10n = AppLocalizations.of(context)!;
    final deviceOptions = <OutputDevice>[
      OutputDevice(
        id: 'default',
        name: l10n.settingsSystemDefault,
        isDefault: true,
        maxChannels: 2,
        defaultSampleRates: Uint32List.fromList([48000]),
      ),
      ...devices.where((d) => d.id != 'default'),
    ];
    final selectedId = deviceOptions.any((d) => d.id == route.deviceId)
        ? route.deviceId
        : 'default';
    final maxChannels = _maxChannelsFor(devices, selectedId);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        SettingsField(
          label: l10n.settingsAudioDevice,
          child: SettingsSelect(
            dialogTitle: l10n.settingsAudioDevice,
            value: selectedId,
            options: [for (final device in deviceOptions) device.id],
            labelBuilder: (id) {
              final device = deviceOptions.firstWhere((d) => d.id == id);
              return device.isDefault
                  ? l10n.settingsSystemDefault
                  : device.name;
            },
            onChanged: (deviceId) => onChanged(
              _clampRoute(
                BusRouteSettings(
                  deviceId: deviceId,
                  leftChannel: route.leftChannel,
                  rightChannel: route.rightChannel,
                  mode: route.mode,
                ),
                _maxChannelsFor(devices, deviceId),
              ),
            ),
          ),
        ),
        const SizedBox(height: 12),
        SettingsField(
          label: l10n.settingsAudioChannelMode,
          child: SettingsSelect(
            dialogTitle: l10n.settingsAudioChannelMode,
            value: route.mode,
            options: BusChannelMode.values,
            labelBuilder: (m) => m == BusChannelMode.stereo
                ? l10n.settingsAudioStereoPair
                : l10n.settingsAudioMonoFold,
            onChanged: (mode) => onChanged(
              BusRouteSettings(
                deviceId: route.deviceId,
                leftChannel: route.leftChannel,
                rightChannel: route.rightChannel,
                mode: mode,
              ),
            ),
          ),
        ),
        const SizedBox(height: 12),
        Row(
          children: [
            Expanded(
              child: SettingsField(
                label: l10n.settingsAudioLeftChannel,
                child: _ChannelStepper(
                  value: route.leftChannel,
                  maxChannels: maxChannels,
                  onChanged: (v) => onChanged(
                    BusRouteSettings(
                      deviceId: route.deviceId,
                      leftChannel: v,
                      rightChannel: route.rightChannel,
                      mode: route.mode,
                    ),
                  ),
                ),
              ),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: SettingsField(
                label: l10n.settingsAudioRightChannel,
                child: _ChannelStepper(
                  value: route.rightChannel,
                  maxChannels: maxChannels,
                  enabled: route.mode == BusChannelMode.stereo,
                  onChanged: (v) => onChanged(
                    BusRouteSettings(
                      deviceId: route.deviceId,
                      leftChannel: route.leftChannel,
                      rightChannel: v,
                      mode: route.mode,
                    ),
                  ),
                ),
              ),
            ),
          ],
        ),
        if (devicesLoading)
          Padding(
            padding: const EdgeInsets.only(top: 8),
            child: Text(
              l10n.settingsAudioLoadingDevices,
              style: theme.typography.body.sm.copyWith(
                color: theme.colors.mutedForeground,
              ),
            ),
          ),
      ],
    );
  }
}

int _maxChannelsFor(List<OutputDevice> devices, String deviceId) {
  final max = resolveAudioOutputDevice(devices, deviceId)?.maxChannels ?? 2;
  return max < 1 ? 1 : max;
}

BusRouteSettings _clampRoute(BusRouteSettings route, int maxChannels) {
  final max = maxChannels < 1 ? 1 : maxChannels;
  final left = route.leftChannel.clamp(1, max);
  var right = route.rightChannel.clamp(1, max);
  var mode = route.mode;
  if (mode == BusChannelMode.stereo && left == right) {
    if (max >= 2) {
      right = left == max ? left - 1 : left + 1;
    } else {
      mode = BusChannelMode.mono;
    }
  }
  return BusRouteSettings(
    deviceId: route.deviceId,
    leftChannel: left,
    rightChannel: right,
    mode: mode,
  );
}

class _ChannelStepper extends StatelessWidget {
  const new({
    required this.value,
    required this.onChanged,
    required this.maxChannels,
    this.enabled = true,
  });

  final int value;
  final ValueChanged<int> onChanged;
  final int maxChannels;
  final bool enabled;

  @override
  Widget build(BuildContext context) {
    final max = maxChannels < 1 ? 1 : maxChannels;
    return Row(
      children: [
        AppButton(
          variant: .outline,
          size: .sm,
          onPress: !enabled || value <= 1 ? null : () => onChanged(value - 1),
          child: const Text('−'),
        ),
        const SizedBox(width: 8),
        Text('$value'),
        const SizedBox(width: 8),
        AppButton(
          variant: .outline,
          size: .sm,
          onPress: !enabled || value >= max ? null : () => onChanged(value + 1),
          child: const Text('+'),
        ),
      ],
    );
  }
}

OutputDevice? resolveAudioOutputDevice(
  List<OutputDevice> devices,
  String deviceId,
) {
  if (deviceId == 'default') {
    for (final device in devices) {
      if (device.isDefault) {
        return device;
      }
    }
    return devices.isEmpty ? null : devices.first;
  }
  for (final device in devices) {
    if (device.id == deviceId) {
      return device;
    }
  }
  return null;
}
