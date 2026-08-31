import 'package:flutter/widgets.dart';
import 'package:gui_flutter/mixer/waveform/layout.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_field.dart';
import 'package:gui_flutter/settings/settings_widgets.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';

const kWaveformVisibleMsPresets = [6000, 12000, 24000, 48000, 60000];

class SettingsWaveformPanel extends StatelessWidget {
  const SettingsWaveformPanel({
    super.key,
    required this.draft,
    required this.onChanged,
  });

  final AppSettings draft;
  final ValueChanged<AppSettings> onChanged;

  @override
  Widget build(BuildContext context) {
    final visibleOptions = {
      ...kWaveformVisibleMsPresets,
      draft.waveformVisibleMs,
    }.toList()..sort();

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const SettingsSectionHeader(
          title: 'Waveform',
          description:
              'RGB mixes low/mid/high into one color. Filtered stacks the three bands.',
        ),
        const SizedBox(height: 20),
        SettingsField(
          label: 'Display mode',
          child: SettingsSelect(
            value: draft.waveformDisplayMode,
            options: WaveformDisplayModeSetting.values,
            labelBuilder: (m) => switch (m) {
              WaveformDisplayModeSetting.rgb => 'RGB',
              WaveformDisplayModeSetting.filtered => 'Filtered',
            },
            onChanged: (m) =>
                onChanged(copyAppSettings(draft, waveformDisplayMode: m)),
          ),
        ),
        const SizedBox(height: 16),
        SettingsField(
          label: 'Default zoom',
          hint: 'Visible window when decks open. Scroll to zoom in-session.',
          child: SettingsSelect<int>(
            value: clampWaveformVisibleMs(draft.waveformVisibleMs),
            options: visibleOptions,
            labelBuilder: (ms) =>
                '${(ms / 1000).toStringAsFixed(ms % 1000 == 0 ? 0 : 1)}s',
            onChanged: (ms) =>
                onChanged(copyAppSettings(draft, waveformVisibleMs: ms)),
          ),
        ),
      ],
    );
  }
}
