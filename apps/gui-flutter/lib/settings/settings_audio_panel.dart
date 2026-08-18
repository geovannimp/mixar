import 'dart:typed_data';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_field.dart';
import 'package:gui_flutter/settings/settings_providers.dart';
import 'package:gui_flutter/settings/settings_widgets.dart';
import 'package:gui_flutter/src/rust/api/engine.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';

class SettingsAudioPanel extends ConsumerWidget {
  const SettingsAudioPanel({
    super.key,
    required this.draft,
    required this.onChanged,
  });

  final AppSettings draft;
  final ValueChanged<AppSettings> onChanged;

  static const _backends = ['cpal', 'auto', 'null'];
  static const _resamplerQualities = ['low', 'medium', 'high'];

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = context.theme;
    final devicesAsync = ref.watch(audioDevicesProvider(draft.backend));
    final deviceList = devicesAsync.value ?? const <OutputDevice>[];
    final sampleRates = _masterBusSampleRates(
      devices: deviceList,
      masterDeviceId: draft.masterBus.deviceId,
      current: draft.sampleRate,
    );

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      spacing: 16,
      children: [
        const SettingsSectionHeader(
          title: 'Audio',
          description: 'Engine output and buses.',
        ),
        SettingsPanel(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            spacing: 16,
            children: [
              Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                spacing: 6,
                children: [
                  SettingsField(
                    label: 'Backend',
                    child: SettingsSelect(
                      value: draft.backend,
                      options: _backends,
                      labelBuilder: (v) => v,
                      onChanged: (backend) =>
                          onChanged(copyAppSettings(draft, backend: backend)),
                    ),
                  ),
                  SettingsToggle(
                    label: 'Low latency',
                    value: draft.lowLatency,
                    onChanged: (v) =>
                        onChanged(copyAppSettings(draft, lowLatency: v)),
                  ),
                ],
              ),
              SettingsField(
                label: 'Sample rate',
                hint: devicesAsync.isLoading
                    ? 'Loading rates for the master output device…'
                    : null,
                child: SettingsSelect(
                  value: draft.sampleRate,
                  options: sampleRates,
                  enabled: !devicesAsync.isLoading && sampleRates.isNotEmpty,
                  labelBuilder: (v) => '$v Hz',
                  onChanged: (sr) =>
                      onChanged(copyAppSettings(draft, sampleRate: sr)),
                ),
              ),
              SettingsField(
                label: 'Resampler quality',
                child: SettingsSelect(
                  value: draft.resamplerQuality,
                  options: _resamplerQualities,
                  labelBuilder: (v) => v,
                  onChanged: (q) =>
                      onChanged(copyAppSettings(draft, resamplerQuality: q)),
                ),
              ),
              SettingsField(
                label: 'Buffer size',
                hint:
                    'Must be a multiple of 64 frames (mixer graph chunk size).',
                child: _BufferSizeSlider(
                  value: draft.bufferSize,
                  onChanged: (v) =>
                      onChanged(copyAppSettings(draft, bufferSize: v)),
                ),
              ),
            ],
          ),
        ),
        SettingsPanel(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            spacing: 16,
            children: [
              Text(
                'Master bus',
                style: theme.typography.body.sm.copyWith(
                  fontWeight: FontWeight.w600,
                ),
              ),
              _BusRouteFields(
                route: draft.masterBus,
                devices: deviceList,
                devicesLoading: devicesAsync.isLoading,
                onChanged: (bus) =>
                    onChanged(copyAppSettings(draft, masterBus: bus)),
              ),
            ],
          ),
        ),
        SettingsPanel(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            spacing: 16,
            children: [
              SettingsToggle(
                label: 'Preview bus (headphones / cue)',
                labelStyle: theme.typography.body.sm.copyWith(
                  fontWeight: FontWeight.w600,
                ),
                value: draft.previewEnabled,
                onChanged: (v) =>
                    onChanged(copyAppSettings(draft, previewEnabled: v)),
              ),
              if (draft.previewEnabled) ...[
                _BusRouteFields(
                  route: draft.previewBus,
                  devices: deviceList,
                  devicesLoading: devicesAsync.isLoading,
                  onChanged: (bus) =>
                      onChanged(copyAppSettings(draft, previewBus: bus)),
                ),
              ],
            ],
          ),
        ),
      ],
    );
  }
}

OutputDevice? _resolveOutputDevice(
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

List<int> _masterBusSampleRates({
  required List<OutputDevice> devices,
  required String masterDeviceId,
  required int current,
}) {
  final device = _resolveOutputDevice(devices, masterDeviceId);
  final rates = <int>[if (device != null) ...device.defaultSampleRates];
  if (!rates.contains(current)) {
    rates.add(current);
  }
  rates.sort();
  return rates;
}

class _BufferSizeSlider extends StatelessWidget {
  const _BufferSizeSlider({required this.value, required this.onChanged});

  static const _min = 64;
  static const _max = 2048;
  static const _step = 64;
  static const _indexMax = (_max - _min) ~/ _step;

  static final _marks = [
    for (var i = 0; i <= _indexMax; i++) FSliderMark(value: i / _indexMax),
  ];

  final int value;
  final ValueChanged<int> onChanged;

  static int _snap(int frames) {
    final snapped = ((frames / _step).round()) * _step;
    return snapped.clamp(_min, _max);
  }

  static double _toNorm(int frames) {
    final index = ((frames - _min) / _step).round().clamp(0, _indexMax);
    return index / _indexMax;
  }

  static int _fromNorm(double norm) {
    final index = (norm * _indexMax).round().clamp(0, _indexMax);
    return _min + index * _step;
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final snapped = _snap(value);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Align(
          alignment: Alignment.centerRight,
          child: Text(
            '$snapped',
            style: theme.typography.body.sm.copyWith(
              fontWeight: FontWeight.w600,
              fontFeatures: const [FontFeature.tabularFigures()],
            ),
          ),
        ),
        const SizedBox(height: 8),
        FSlider(
          control: .liftedDiscrete(
            value: FSliderValue(max: _toNorm(snapped)),
            onChange: (v) => onChanged(_fromNorm(v.max)),
          ),
          marks: _marks,
          tooltipBuilder: (_, norm) => Text('${_fromNorm(norm)}'),
          semanticValueFormatterCallback: (norm) => '${_fromNorm(norm)} frames',
        ),
      ],
    );
  }
}

class _BusRouteFields extends StatelessWidget {
  const _BusRouteFields({
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
    final deviceOptions = <OutputDevice>[
      OutputDevice(
        id: 'default',
        name: 'System default',
        isDefault: true,
        maxChannels: 2,
        defaultSampleRates: Uint32List.fromList([48000]),
      ),
      ...devices.where((d) => d.id != 'default'),
    ];
    final selectedId = deviceOptions.any((d) => d.id == route.deviceId)
        ? route.deviceId
        : 'default';

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        SettingsField(
          label: 'Device',
          child: SettingsSelect(
            value: selectedId,
            options: [for (final device in deviceOptions) device.id],
            labelBuilder: (id) {
              final device = deviceOptions.firstWhere((d) => d.id == id);
              return device.isDefault ? 'System default' : device.name;
            },
            onChanged: (deviceId) => onChanged(
              BusRouteSettings(
                deviceId: deviceId,
                leftChannel: route.leftChannel,
                rightChannel: route.rightChannel,
                mode: route.mode,
              ),
            ),
          ),
        ),
        const SizedBox(height: 12),
        SettingsField(
          label: 'Channel mode',
          child: SettingsSelect(
            value: route.mode,
            options: BusChannelMode.values,
            labelBuilder: (m) =>
                m == BusChannelMode.stereo ? 'Stereo pair' : 'Mono (fold L+R)',
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
                label: 'Left channel',
                child: _ChannelStepper(
                  value: route.leftChannel,
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
                label: 'Right channel',
                child: _ChannelStepper(
                  value: route.rightChannel,
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
              'Loading devices…',
              style: theme.typography.body.sm.copyWith(
                color: theme.colors.mutedForeground,
              ),
            ),
          ),
      ],
    );
  }
}

class _ChannelStepper extends StatelessWidget {
  const _ChannelStepper({
    required this.value,
    required this.onChanged,
    this.enabled = true,
  });

  final int value;
  final ValueChanged<int> onChanged;
  final bool enabled;

  @override
  Widget build(BuildContext context) {
    return Row(
      children: [
        FButton(
          variant: .outline,
          size: .sm,
          onPress: !enabled || value <= 1 ? null : () => onChanged(value - 1),
          child: const Text('−'),
        ),
        const SizedBox(width: 8),
        Text('$value'),
        const SizedBox(width: 8),
        FButton(
          variant: .outline,
          size: .sm,
          onPress: enabled ? () => onChanged(value + 1) : null,
          child: const Text('+'),
        ),
      ],
    );
  }
}
