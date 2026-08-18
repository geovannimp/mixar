import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/mixer/waveform/spectral_color.dart';
import 'package:gui_flutter/mixer/waveform/waveform_providers.dart';
import 'package:gui_flutter/settings/settings_field.dart';
import 'package:gui_flutter/settings/settings_widgets.dart';

class SettingsWaveformPanel extends ConsumerWidget {
  const SettingsWaveformPanel({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final mode = ref.watch(waveformDisplayModeProvider);

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
            value: mode,
            options: WaveformDisplayMode.values,
            labelBuilder: (m) => switch (m) {
              WaveformDisplayMode.rgb => 'RGB',
              WaveformDisplayMode.filtered => 'Filtered',
            },
            onChanged: (m) =>
                ref.read(waveformDisplayModeProvider.notifier).set(m),
          ),
        ),
      ],
    );
  }
}
