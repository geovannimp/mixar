import 'package:flutter/widgets.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_field.dart';
import 'package:gui_flutter/settings/settings_widgets.dart';
import 'package:gui_flutter/shell/mixar_input.dart';
import 'package:gui_flutter/shell/mixar_slider.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';

class SettingsSessionPanel extends StatelessWidget {
  const new({required this.draft, required this.onChanged, super.key});

  final AppSettings draft;
  final ValueChanged<AppSettings> onChanged;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      spacing: 16,
      children: [
        SettingsSectionHeader(
          title: l10n.settingsSectionSession,
          description: l10n.settingsSessionDescription,
        ),
        SettingsPanel(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            spacing: 16,
            children: [
              SettingsSectionHeader(
                title: l10n.settingsSessionHistoryTitle,
                description: l10n.settingsSessionHistoryDescription,
              ),
              SettingsToggle(
                label: l10n.settingsSessionRecordHistory,
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
                      label: l10n.settingsSessionIdleTimeout,
                      hint: l10n.settingsSessionIdleTimeoutHint,
                      child: MixarInput(
                        initialValue: '${draft.historySessionIdleMinutes}',
                        trailing: _suffixLabel(
                          context,
                          l10n.settingsSessionMinutes,
                        ),
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
                      label: l10n.settingsSessionMinPlayDuration,
                      hint: l10n.settingsSessionMinPlayDurationHint,
                      child: MixarInput(
                        initialValue: '${draft.historyMinPlaySeconds}',
                        trailing: _suffixLabel(
                          context,
                          l10n.settingsSessionSeconds,
                        ),
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
                label: l10n.settingsSessionMinDeckVolume,
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
  const new({required this.value, required this.onChanged});

  final double value;
  final ValueChanged<double> onChanged;

  static double _snap(double volume) {
    return (volume.clamp(0.0, 1.0) * 100).round() / 100.0;
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final l10n = AppLocalizations.of(context)!;
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
          semanticFormatterCallback: (v) =>
              l10n.settingsSessionPercentSemantics((v * 100).round()),
        ),
      ],
    );
  }
}
