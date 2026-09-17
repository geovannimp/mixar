import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_field.dart';
import 'package:gui_flutter/settings/settings_widgets.dart';
import 'package:gui_flutter/shell/mixar_input.dart';
import 'package:gui_flutter/shell/mixar_slider.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';

class SettingsSessionPanel extends StatelessWidget {
  const SettingsSessionPanel({
    super.key,
    required this.draft,
    required this.onChanged,
  });

  final AppSettings draft;
  final ValueChanged<AppSettings> onChanged;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      spacing: 16,
      children: [
        const SettingsSectionHeader(
          title: 'Session',
          description: 'Performance history and session boundaries.',
        ),
        SettingsPanel(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            spacing: 16,
            children: [
              const SettingsSectionHeader(
                title: 'Performance history',
                description:
                    'Log deck playback to XSPF session files under app support.',
              ),
              SettingsToggle(
                label: 'Record performance history',
                value: draft.historyEnabled,
                onChanged: (enabled) =>
                    onChanged(copyAppSettings(draft, historyEnabled: enabled)),
              ),
              const SizedBox(height: 0),
              Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                spacing: 12,
                children: [
                  Expanded(
                    child: SettingsField(
                      label: 'Session idle timeout',
                      hint:
                          'Close after this long with no qualifying deck output.',
                      child: MixarInput(
                        initialValue: '${draft.historySessionIdleMinutes}',
                        trailing: _suffixLabel(context, 'minutes'),
                        onChanged: (text) {
                          final parsed = int.tryParse(text.trim());
                          if (parsed != null && parsed > 0) {
                            onChanged(
                              copyAppSettings(
                                draft,
                                historySessionIdleMinutes: parsed,
                              ),
                            );
                          }
                        },
                      ),
                    ),
                  ),
                  Expanded(
                    child: SettingsField(
                      label: 'Minimum play duration',
                      hint:
                          'Commit entries after this much qualifying playback.',
                      child: MixarInput(
                        initialValue: '${draft.historyMinPlaySeconds}',
                        trailing: _suffixLabel(context, 'seconds'),
                        onChanged: (text) {
                          final parsed = int.tryParse(text.trim());
                          if (parsed != null && parsed > 0) {
                            onChanged(
                              copyAppSettings(
                                draft,
                                historyMinPlaySeconds: parsed,
                              ),
                            );
                          }
                        },
                      ),
                    ),
                  ),
                ],
              ),
              SettingsField(
                label: 'Minimum effective deck volume',
                child: _MinDeckVolumeSlider(
                  value: draft.historyMinDeckVolume,
                  onChanged: (volume) => onChanged(
                    copyAppSettings(draft, historyMinDeckVolume: volume),
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

Widget _suffixLabel(BuildContext context, String text) {
  final theme = context.theme;
  return Padding(
    padding: const EdgeInsets.only(right: 12),
    child: Text(
      text,
      style: theme.typography.body.sm.copyWith(
        color: theme.colors.mutedForeground,
      ),
    ),
  );
}

class _MinDeckVolumeSlider extends StatelessWidget {
  const _MinDeckVolumeSlider({required this.value, required this.onChanged});

  final double value;
  final ValueChanged<double> onChanged;

  static double _snap(double volume) {
    return (volume.clamp(0.0, 1.0) * 100).round() / 100.0;
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final snapped = _snap(value);
    final label = '${(snapped * 100).toStringAsFixed(0)}%';

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Align(
          alignment: Alignment.centerRight,
          child: Text(
            label,
            style: theme.typography.body.sm.copyWith(
              fontWeight: FontWeight.w600,
              fontFeatures: const [FontFeature.tabularFigures()],
            ),
          ),
        ),
        const SizedBox(height: 8),
        MixarSlider(
          value: snapped,
          divisions: 100,
          onChanged: (v) => onChanged(_snap(v)),
          semanticFormatterCallback: (v) => '${(v * 100).round()} percent',
        ),
      ],
    );
  }
}
