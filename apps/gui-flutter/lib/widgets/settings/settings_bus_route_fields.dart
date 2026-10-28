import 'dart:typed_data';

import 'package:flutter/widgets.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/src/rust/api/engine.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_checkbox.dart';
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
    this.siblingBus,
  });

  final BusRouteSettings route;
  final List<OutputDevice> devices;
  final bool devicesLoading;
  final ValueChanged<BusRouteSettings> onChanged;

  /// The other live bus, when it exists. Buses on one device share one stream, so
  /// an explicit rate there decides the rate a "Device default" bus actually runs at.
  final BusRouteSettings? siblingBus;

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
        // Rates come from the resolved device, not this placeholder entry.
        defaultSampleRates: Uint32List(0),
        sampleRateRanges: const [],
      ),
      ...devices.where((d) => d.id != 'default'),
    ];
    final selectedId = deviceOptions.any((d) => d.id == route.deviceId)
        ? route.deviceId
        : 'default';
    final maxChannels = _maxChannelsFor(devices, selectedId);
    final busDevice = resolveAudioOutputDevice(devices, selectedId);
    final rateOptions = _rateOptions(busDevice, route.sampleRate);
    // A sibling on the same device with an explicit rate pins the shared stream.
    final sibling = siblingBus;
    final siblingDevice = sibling == null
        ? null
        : resolveAudioOutputDevice(devices, sibling.deviceId)?.id ??
              sibling.deviceId;
    final busDeviceId = busDevice?.id ?? selectedId;
    final inheritedRate =
        sibling != null &&
            sibling.sampleRate != null &&
            siblingDevice == busDeviceId
        ? sibling.sampleRate
        : null;

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
                  sampleRate: _retainedRate(
                    route.sampleRate,
                    resolveAudioOutputDevice(devices, deviceId),
                  ),
                  lowLatency: route.lowLatency,
                ),
                _maxChannelsFor(devices, deviceId),
              ),
            ),
          ),
        ),
        const SizedBox(height: 12),
        SettingsField(
          label: l10n.settingsAudioSampleRate,
          hint: devicesLoading
              ? l10n.settingsAudioSampleRateLoading
              : l10n.settingsAudioSampleRateHint,
          child: SettingsSelect<int>(
            dialogTitle: l10n.settingsAudioSampleRate,
            value: route.sampleRate ?? 0,
            options: rateOptions,
            enabled: !devicesLoading && rateOptions.length > 1,
            labelBuilder: (rate) =>
                _rateLabel(l10n, busDevice, rate, inheritedRate),
            onChanged: (rate) => onChanged(
              BusRouteSettings(
                deviceId: route.deviceId,
                leftChannel: route.leftChannel,
                rightChannel: route.rightChannel,
                mode: route.mode,
                // `0` is the "follow the device mix rate" token.
                sampleRate: rate == 0 ? null : rate,
                lowLatency: route.lowLatency,
              ),
            ),
          ),
        ),
        const SizedBox(height: 12),
        SettingsCheckbox(
          label: l10n.settingsAudioLowLatency,
          value: route.lowLatency,
          onChanged: (lowLatency) => onChanged(
            BusRouteSettings(
              deviceId: route.deviceId,
              leftChannel: route.leftChannel,
              rightChannel: route.rightChannel,
              mode: route.mode,
              sampleRate: route.sampleRate,
              lowLatency: lowLatency,
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
                sampleRate: route.sampleRate,
                lowLatency: route.lowLatency,
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
                      sampleRate: route.sampleRate,
                      lowLatency: route.lowLatency,
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
                      sampleRate: route.sampleRate,
                      lowLatency: route.lowLatency,
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
    sampleRate: route.sampleRate,
    lowLatency: route.lowLatency,
  );
}

/// Keep an explicit rate only when the newly selected device can open it: a rate
/// the device cannot open would save cleanly and then refuse to start the engine.
/// Checked against the device's supported spans, because the advertised list is
/// only their endpoints — a rate may sit inside a range without being listed.
int? _retainedRate(int? rate, OutputDevice? device) {
  if (rate == null || device == null) {
    return rate;
  }
  if (device.sampleRateRanges.isEmpty) {
    return device.defaultSampleRates.contains(rate) ? rate : null;
  }
  return device.sampleRateRanges.any(
        (range) => rate >= range.min && rate <= range.max,
      )
      ? rate
      : null;
}

/// Rate options for a bus: `0` is the "device default" token (no rate is ever 0),
/// then the rates the device advertises. A rate the bus already has stays listed so
/// an older value remains selectable even when the device no longer advertises it.
///
/// The token is not `null` because a select cannot distinguish a `null` option from
/// dismissal: mobile pops `null` on cancel and desktop ignores it, which would make
/// "back to device default" impossible after choosing a rate.
List<int> _rateOptions(OutputDevice? device, int? current) {
  final rates = <int>[0, if (device != null) ...device.defaultSampleRates];
  if (current != null && !rates.contains(current)) {
    rates.add(current);
  }
  rates.sort();
  return rates;
}

/// Label for a rate option. With the default token selected, report the rate the
/// stream will actually run at: a sibling bus on the same device with an explicit
/// rate decides it, otherwise the device's mix rate does.
String _rateLabel(
  AppLocalizations l10n,
  OutputDevice? device,
  int rate,
  int? inheritedRate,
) {
  if (rate != 0) {
    return l10n.settingsAudioSampleRateHz(rate);
  }
  final effective = inheritedRate ?? device?.mixSampleRate;
  return effective == null
      ? l10n.settingsAudioSampleRateDeviceDefault
      : l10n.settingsAudioSampleRateDeviceDefaultWithRate(effective);
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
