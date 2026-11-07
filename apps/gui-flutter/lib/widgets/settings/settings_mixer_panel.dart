import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/theme/mixar_button_style.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_field.dart';
import 'package:gui_flutter/widgets/settings/settings_header_switch.dart';
import 'package:gui_flutter/widgets/settings/settings_numeric_stepper.dart';
import 'package:gui_flutter/widgets/settings/settings_section_body.dart';
import 'package:gui_flutter/widgets/settings/settings_section_card.dart';
import 'package:gui_flutter/widgets/settings/settings_select.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

class SettingsMixerPanel extends StatelessWidget {
  const new({
    required this.draft,
    required this.onChanged,
    this.capability,
    this.benchmarkRunning = false,
    this.benchmarkFraction,
    this.benchmarkError,
    this.onRunBenchmark,
    super.key,
  });

  final AppSettings draft;
  final ValueChanged<AppSettings> onChanged;

  /// Last measured CPU capability; `null` until this device has run a probe.
  final RealtimeStemsCapability? capability;
  final bool benchmarkRunning;

  /// 0…1 while [benchmarkRunning]; `null` hides the percentage.
  final double? benchmarkFraction;
  final String? benchmarkError;

  /// Runs the probe. When omitted the capability gate is inactive (widgets
  /// mounted without the settings page, e.g. focused panel tests).
  final VoidCallback? onRunBenchmark;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final theme = context.theme;
    final capability = this.capability;
    // Live stems stay gated on the measurement whenever the probe is wired:
    // a missing, stale, or below-margin result keeps the switch off.
    final gated = onRunBenchmark != null;
    final allowed =
        !gated || (!benchmarkRunning && (capability?.allowed ?? false));

    final status = _statusText(
      l10n: l10n,
      capability: capability,
      running: benchmarkRunning,
      fraction: benchmarkFraction,
      error: benchmarkError,
      muted: theme.colors.mutedForeground,
      destructive: theme.colors.destructive,
    );

    return SettingsSectionBody(
      title: l10n.settingsSectionMixer,
      description: l10n.settingsMixerDescription,
      cards: [
        SettingsSectionCard(
          title: l10n.settingsMixerNormalizerTitle,
          subtitle: l10n.settingsMixerNormalizerDescription,
          trailing: SettingsHeaderSwitch(
            value: draft.volumeNormalizerEnabled,
            semanticsLabel: l10n.settingsMixerVolumeNormalizer,
            onChanged: (v) =>
                onChanged(copyAppSettings(draft, volumeNormalizerEnabled: v)),
          ),
          child: draft.volumeNormalizerEnabled
              ? SettingsField(
                  label: l10n.settingsMixerTargetLufs,
                  child: SettingsNumericStepper(
                    value: draft.targetLufs,
                    min: kMinTargetLufs,
                    max: kMaxTargetLufs,
                    step: 0.5,
                    format: (v) => v.toStringAsFixed(1),
                    onChanged: (v) =>
                        onChanged(copyAppSettings(draft, targetLufs: v)),
                  ),
                )
              : null,
        ),
        SettingsSectionCard(
          title: l10n.settingsMixerSamplerRoutingTitle,
          subtitle: l10n.settingsMixerSamplerRoutingDescription,
          child: SettingsField(
            label: l10n.settingsMixerSamplerStripRoute,
            child: SettingsSelect(
              dialogTitle: l10n.settingsMixerSamplerStripRoute,
              value: draft.samplerStripRoute,
              options: SamplerStripRouteSettingFrb.values,
              labelBuilder: (m) => m == SamplerStripRouteSettingFrb.before
                  ? l10n.settingsMixerSamplerStripBefore
                  : l10n.settingsMixerSamplerStripAfter,
              onChanged: (m) =>
                  onChanged(copyAppSettings(draft, samplerStripRoute: m)),
            ),
          ),
        ),
        SettingsSectionCard(
          title: l10n.settingsMixerRealtimeStemsTitle,
          subtitle: l10n.settingsMixerRealtimeStemsDescription,
          subtitleBuilder: (style) => Text.rich(
            TextSpan(
              children: [
                TextSpan(text: l10n.settingsMixerRealtimeStemsDescription),
                TextSpan(
                  text: '\n${l10n.settingsMixerRealtimeStemsNotRecommended}',
                  style: style.copyWith(fontWeight: FontWeight.w700),
                ),
                TextSpan(text: ' ${l10n.settingsMixerRealtimeStemsWarning}'),
              ],
            ),
            style: style,
          ),
          trailing: SettingsHeaderSwitch(
            value: draft.realtimeStems && allowed,
            enabled: allowed,
            semanticsLabel: l10n.settingsMixerRealtimeStemsTitle,
            onChanged: (enabled) =>
                onChanged(copyAppSettings(draft, realtimeStems: enabled)),
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            spacing: 12,
            children: [
              if (status != null)
                Text(
                  status.text,
                  style: theme.typography.body.sm.copyWith(color: status.color),
                ),
              if (onRunBenchmark != null)
                AppButton(
                  size: MixarButtonSize.sm,
                  mainAxisSize: MainAxisSize.min,
                  onPress: benchmarkRunning ? null : onRunBenchmark,
                  child: Text(
                    capability == null
                        ? l10n.settingsMixerRealtimeStemsRunBenchmark
                        : l10n.settingsMixerRealtimeStemsRerunBenchmark,
                  ),
                ),
            ],
          ),
        ),
      ],
    );
  }

  ({String text, Color color})? _statusText({
    required AppLocalizations l10n,
    required RealtimeStemsCapability? capability,
    required bool running,
    required double? fraction,
    required String? error,
    required Color muted,
    required Color destructive,
  }) {
    if (error != null) {
      return (
        text: l10n.settingsMixerRealtimeStemsBenchmarkFailed(error),
        color: destructive,
      );
    }
    if (running) {
      final percent = fraction == null ? null : (fraction * 100).round();
      return (
        text: percent == null
            ? l10n.settingsMixerRealtimeStemsBenchmarking
            : '${l10n.settingsMixerRealtimeStemsBenchmarking} $percent%',
        color: muted,
      );
    }
    final measured = capability;
    if (measured == null) {
      return (
        text: l10n.settingsMixerRealtimeStemsBenchmarkNeeded,
        color: muted,
      );
    }
    final realtime = measured.realtime.toStringAsFixed(2);
    if (measured.allowed) {
      return (
        text: l10n.settingsMixerRealtimeStemsMeasured(
          realtime,
          measured.avgHopMs.toStringAsFixed(2),
        ),
        color: muted,
      );
    }
    return (
      text: l10n.settingsMixerRealtimeStemsTooSlow(
        realtime,
        measured.minRealtime.toStringAsFixed(1),
      ),
      color: destructive,
    );
  }
}
