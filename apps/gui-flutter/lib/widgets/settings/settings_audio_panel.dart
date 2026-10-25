import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/src/rust/api/engine.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_bus_route_fields.dart';
import 'package:gui_flutter/widgets/settings/settings_checkbox.dart';
import 'package:gui_flutter/widgets/settings/settings_field.dart';
import 'package:gui_flutter/widgets/settings/settings_glow_flash.dart';
import 'package:gui_flutter/widgets/settings/settings_header_switch.dart';
import 'package:gui_flutter/widgets/settings/settings_section_card.dart';
import 'package:gui_flutter/widgets/settings/settings_section_header.dart';
import 'package:gui_flutter/widgets/settings/settings_select.dart';
import 'package:gui_flutter/widgets/shell/mixar_slider.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

class SettingsAudioPanel extends ConsumerWidget {
  const new({
    required this.draft,
    required this.onChanged,
    required this.previewBusCardKey,
    this.highlightPreviewBus = false,
    super.key,
  });

  final AppSettings draft;
  final ValueChanged<AppSettings> onChanged;
  final GlobalKey previewBusCardKey;
  final bool highlightPreviewBus;

  static const _resamplerQualities = ['low', 'medium', 'high'];

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context)!;
    final devicesAsync = ref.watch(audioDevicesProvider);
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
        SettingsSectionHeader(
          title: l10n.settingsSectionAudio,
          description: l10n.settingsAudioDescription,
        ),
        SettingsSectionCard(
          title: l10n.settingsAudioEngineTitle,
          subtitle: l10n.settingsAudioEngineDescription,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            spacing: 16,
            children: [
              SettingsCheckbox(
                label: l10n.settingsAudioLowLatency,
                value: draft.lowLatency,
                onChanged: (v) =>
                    onChanged(copyAppSettings(draft, lowLatency: v)),
              ),
              SettingsField(
                label: l10n.settingsAudioSampleRate,
                hint: devicesAsync.isLoading
                    ? l10n.settingsAudioSampleRateLoading
                    : null,
                child: SettingsSelect(
                  dialogTitle: l10n.settingsAudioSampleRate,
                  value: draft.sampleRate,
                  options: sampleRates,
                  enabled: !devicesAsync.isLoading && sampleRates.isNotEmpty,
                  labelBuilder: (v) => '$v Hz',
                  onChanged: (sr) =>
                      onChanged(copyAppSettings(draft, sampleRate: sr)),
                ),
              ),
              SettingsField(
                label: l10n.settingsAudioResamplerQuality,
                child: SettingsSelect(
                  dialogTitle: l10n.settingsAudioResamplerQuality,
                  value: draft.resamplerQuality,
                  options: _resamplerQualities,
                  labelBuilder: (v) => switch (v) {
                    'low' => l10n.settingsAudioResamplerLow,
                    'high' => l10n.settingsAudioResamplerHigh,
                    _ => l10n.settingsAudioResamplerMedium,
                  },
                  onChanged: (q) =>
                      onChanged(copyAppSettings(draft, resamplerQuality: q)),
                ),
              ),
              SettingsField(
                label: l10n.settingsAudioBufferSize,
                hint: l10n.settingsAudioBufferSizeHint,
                child: _BufferSizeSlider(
                  value: draft.bufferSize,
                  onChanged: (v) =>
                      onChanged(copyAppSettings(draft, bufferSize: v)),
                ),
              ),
            ],
          ),
        ),
        SettingsSectionCard(
          title: l10n.settingsAudioMasterBus,
          subtitle: l10n.settingsAudioMasterBusDescription,
          child: SettingsBusRouteFields(
            route: draft.masterBus,
            devices: deviceList,
            devicesLoading: devicesAsync.isLoading,
            onChanged: (bus) =>
                onChanged(copyAppSettings(draft, masterBus: bus)),
          ),
        ),
        SettingsGlowFlash(
          key: previewBusCardKey,
          active: highlightPreviewBus,
          child: SettingsSectionCard(
            title: l10n.settingsAudioPreviewBusTitle,
            subtitle: l10n.settingsAudioPreviewBusDescription,
            trailing: SettingsHeaderSwitch(
              value: draft.previewEnabled,
              semanticsLabel: l10n.settingsAudioPreviewBus,
              onChanged: (v) =>
                  onChanged(copyAppSettings(draft, previewEnabled: v)),
            ),
            child: draft.previewEnabled
                ? SettingsBusRouteFields(
                    route: draft.previewBus,
                    devices: deviceList,
                    devicesLoading: devicesAsync.isLoading,
                    onChanged: (bus) =>
                        onChanged(copyAppSettings(draft, previewBus: bus)),
                  )
                : null,
          ),
        ),
      ],
    );
  }
}

List<int> _masterBusSampleRates({
  required List<OutputDevice> devices,
  required String masterDeviceId,
  required int current,
}) {
  final device = resolveAudioOutputDevice(devices, masterDeviceId);
  final rates = <int>[if (device != null) ...device.defaultSampleRates];
  if (!rates.contains(current)) {
    rates.add(current);
  }
  rates.sort();
  return rates;
}

class _BufferSizeSlider extends StatelessWidget {
  const new({required this.value, required this.onChanged});

  static const _min = 64;
  static const _max = 2048;
  static const _step = 64;
  static const int _indexMax = (_max - _min) ~/ _step;

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
    final l10n = AppLocalizations.of(context)!;
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
        MixarSlider(
          value: _toNorm(snapped),
          divisions: _indexMax,
          onChanged: (norm) => onChanged(_fromNorm(norm)),
          semanticFormatterCallback: (norm) =>
              l10n.settingsAudioBufferSizeSemantics(_fromNorm(norm)),
        ),
      ],
    );
  }
}
