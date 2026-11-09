import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/models/curve_editor.dart';
import 'package:gui_flutter/core/theme/deck_accent.dart';
import 'package:gui_flutter/core/theme/mixar_button_style.dart';
import 'package:gui_flutter/core/theme/mixar_colors.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/src/rust/api/event_mirrors.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_checkbox.dart';
import 'package:gui_flutter/widgets/settings/settings_field.dart';
import 'package:gui_flutter/widgets/settings/settings_header_switch.dart';
import 'package:gui_flutter/widgets/settings/settings_numeric_stepper.dart';
import 'package:gui_flutter/widgets/settings/settings_section_body.dart';
import 'package:gui_flutter/widgets/settings/settings_section_card.dart';
import 'package:gui_flutter/widgets/settings/settings_select.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/m_tappable.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

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
    // the switch replaces the benchmark action only after a passing run.
    final gated = onRunBenchmark != null;
    final passed = !benchmarkRunning && (capability?.allowed ?? false);
    // Once the feature is on there is nothing left to verify: keep the switch
    // and drop both the benchmark action and every status badge. Enabling is
    // gated when the switch is turned on, not on every launch.
    final enabled = draft.realtimeStems;

    final status = enabled
        ? null
        : _status(
            l10n: l10n,
            capability: capability,
            running: benchmarkRunning,
            error: benchmarkError,
            theme: theme,
          );

    return SettingsSectionBody(
      title: l10n.settingsSectionMixer,
      description: l10n.settingsMixerDescription,
      cards: [
        SettingsSectionCard(
          title: l10n.settingsMixerCrossfaderTitle,
          subtitle: l10n.settingsMixerCrossfaderDescription,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            spacing: 16,
            children: [
              SettingsField(
                label: l10n.settingsMixerCrossfaderCurve,
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  spacing: 8,
                  children: [
                    for (final id in _curveOptions(draft))
                      _CurveChoice(
                        id: id,
                        label: _curveLabel(draft, id, l10n),
                        selected: draft.defaultCrossfaderCurve == id,
                        channels: _previewChannels(draft, id),
                        onSelected: (id) => onChanged(
                          copyAppSettings(draft, defaultCrossfaderCurve: id),
                        ),
                      ),
                  ],
                ),
              ),
              SettingsCheckbox(
                label: l10n.settingsMixerCrossfaderInvert,
                value: draft.crossfaderBehavior.hamster,
                onChanged: (value) => onChanged(
                  copyAppSettings(
                    draft,
                    crossfaderBehavior: CrossfaderBehaviorSetting(
                      disabled: draft.crossfaderBehavior.disabled,
                      hamster: value,
                    ),
                  ),
                ),
              ),
              SettingsCheckbox(
                label: l10n.settingsMixerCrossfaderDisable,
                value: draft.crossfaderBehavior.disabled,
                onChanged: (value) => onChanged(
                  copyAppSettings(
                    draft,
                    crossfaderBehavior: CrossfaderBehaviorSetting(
                      disabled: value,
                      hamster: draft.crossfaderBehavior.hamster,
                    ),
                  ),
                ),
              ),
            ],
          ),
        ),
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
          trailing: !gated || passed || enabled
              ? SettingsHeaderSwitch(
                  value: draft.realtimeStems,
                  semanticsLabel: l10n.settingsMixerRealtimeStemsTitle,
                  onChanged: (next) =>
                      onChanged(copyAppSettings(draft, realtimeStems: next)),
                )
              : _BenchmarkAction(
                  l10n: l10n,
                  capability: capability,
                  running: benchmarkRunning,
                  fraction: benchmarkFraction,
                  onRun: onRunBenchmark,
                ),
          child: status == null ? null : _CapabilityBadge(status: status),
        ),
      ],
    );
  }

  _CapabilityStatus? _status({
    required AppLocalizations l10n,
    required RealtimeStemsCapability? capability,
    required bool running,
    required String? error,
    required MixarThemeData theme,
  }) {
    if (running) {
      // The action carries the progress; no second status line while it runs.
      return null;
    }
    final colors = theme.colors;
    if (error != null) {
      return _blockedStatus(
        colors: colors,
        icon: LucideIcons.circleAlert,
        text: l10n.settingsMixerRealtimeStemsBenchmarkFailed(error),
      );
    }
    final measured = capability;
    if (measured == null) {
      // No benchmark result yet: the action button is the whole message.
      return null;
    }
    if (measured.allowed) {
      final dark = colors.brightness == Brightness.dark;
      return _CapabilityStatus(
        icon: LucideIcons.badgeCheck,
        text: l10n.settingsMixerRealtimeStemsDeviceReady,
        color: dark ? _readyDark : _readyLight,
        fill: dark ? _readyDarkFill : _readyLightFill,
        border: dark ? _readyDarkBorder : _readyLightBorder,
      );
    }
    return _blockedStatus(
      colors: colors,
      icon: LucideIcons.ban,
      text: l10n.settingsMixerRealtimeStemsDeviceTooSlow,
    );
  }

  /// Shared red verdict for a failed probe or a below-margin result.
  _CapabilityStatus _blockedStatus({
    required MixarColors colors,
    required IconData icon,
    required String text,
  }) {
    return _CapabilityStatus(
      icon: icon,
      text: text,
      color: colors.destructive,
      fill: colors.destructive.withValues(alpha: 0.12),
      border: colors.destructive.withValues(alpha: 0.4),
    );
  }

  /// Preset ids plus the user's custom groups and curve sets.
  ///
  /// A saved id that no longer exists is kept at the top so the picker shows
  /// the real value instead of silently selecting a different curve.
  List<String> _curveOptions(AppSettings draft) {
    final options = <String>[
      ...kCrossfaderCurvePresets,
      for (final group in draft.curveLibrary.groups) group.id,
      for (final set in draft.curveLibrary.sets) set.id,
    ];
    if (!options.contains(draft.defaultCrossfaderCurve)) {
      options.insert(0, draft.defaultCrossfaderCurve);
    }
    return options;
  }

  /// Display name for a curve id: preset name, custom label, or the id itself.
  String _curveLabel(AppSettings draft, String id, AppLocalizations l10n) {
    final preset = switch (id) {
      'constant_power' => l10n.settingsCrossfaderCurveConstantPower,
      'linear' => l10n.settingsCrossfaderCurveLinear,
      'fast_cut' => l10n.settingsCrossfaderCurveFastCut,
      _ => null,
    };
    if (preset != null) {
      return preset;
    }
    for (final group in draft.curveLibrary.groups) {
      if (group.id == id) {
        return group.displayName.isEmpty ? id : group.displayName;
      }
    }
    for (final set in draft.curveLibrary.sets) {
      if (set.id == id) {
        return set.displayName.isEmpty ? id : set.displayName;
      }
    }
    return id;
  }

  /// Sample a curve id into the A/B channels a preview draws.
  List<List<double>> _previewChannels(AppSettings draft, String id) {
    const samples = 48;
    List<double> at(double x) {
      if (kCrossfaderPresetIds.contains(id)) {
        return evaluateCurvePreset(id, null, x);
      }
      for (final group in draft.curveLibrary.groups) {
        if (group.id == id) {
          return evaluateCurveGroup(group, x);
        }
      }
      for (final set in draft.curveLibrary.sets) {
        if (set.id == id) {
          return evaluateCurveSet(set, draft.curveLibrary, 0.5, x);
        }
      }
      return const [0, 0];
    }

    final samplesAtX = [
      for (var i = 0; i < samples; i++) at(i / (samples - 1)),
    ];
    return [
      [for (final values in samplesAtX) values[0]],
      [for (final values in samplesAtX) values[1]],
    ];
  }
}

/// Replaces the switch until the device has a passing measurement.
///
/// While the probe runs the button stays disabled and carries its progress, so
/// the action and its feedback live in the same slot.
class _BenchmarkAction extends StatelessWidget {
  const new({
    required this.l10n,
    required this.capability,
    required this.running,
    required this.fraction,
    required this.onRun,
  });

  final AppLocalizations l10n;
  final RealtimeStemsCapability? capability;
  final bool running;
  final double? fraction;
  final VoidCallback? onRun;

  @override
  Widget build(BuildContext context) {
    final percent = fraction == null ? null : (fraction! * 100).round();
    final label = running
        ? (percent == null
              ? l10n.settingsMixerRealtimeStemsBenchmarking
              : '${l10n.settingsMixerRealtimeStemsBenchmarking} $percent%')
        : (capability == null
              ? l10n.settingsMixerRealtimeStemsRunBenchmark
              : l10n.settingsMixerRealtimeStemsRerunBenchmark);
    return AppButton(
      size: MixarButtonSize.sm,
      mainAxisSize: MainAxisSize.min,
      onPress: running ? null : onRun,
      child: Text(label),
    );
  }
}

/// Semantic success green for the capability verdict.
///
/// The palette's brand signal is teal and it intentionally ships no green
/// ladder, so this status colour stays local (like the library's live dot).
const _readyLight = Color(0xFF15803D);
const _readyLightFill = Color(0xFFF0FDF4);
const _readyLightBorder = Color(0xFFBBF7D0);
const _readyDark = Color(0xFF4ADE80);
const _readyDarkFill = Color(0xFF052E16);
const _readyDarkBorder = Color(0xFF166534);

class _CapabilityStatus {
  const new({
    required this.icon,
    required this.text,
    required this.color,
    required this.fill,
    required this.border,
  });

  final IconData icon;
  final String text;
  final Color color;
  final Color fill;
  final Color border;
}

/// One radio-style default-curve choice with a small A/B preview.
class _CurveChoice extends StatelessWidget {
  const new({
    required this.id,
    required this.label,
    required this.selected,
    required this.channels,
    required this.onSelected,
  });

  final String id;
  final String label;
  final bool selected;
  final List<List<double>> channels;
  final ValueChanged<String> onSelected;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final colors = [
      deckInk(DeckAccent.a, theme.colors.brightness),
      deckInk(DeckAccent.b, theme.colors.brightness),
    ];
    return MTappable(
      onPress: () => onSelected(id),
      selected: selected,
      semanticsLabel: label,
      builder: (context, state) {
        final border = selected ? theme.colors.primary : theme.colors.border;
        return DecoratedBox(
          decoration: BoxDecoration(
            color: selected ? theme.colors.selection : theme.colors.card,
            border: Border.all(color: border, width: selected ? 1.5 : 1),
            borderRadius: theme.style.borderRadius.md,
          ),
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
            child: Row(
              children: [
                _RadioMark(selected: selected),
                const SizedBox(width: 10),
                Expanded(
                  child: Text(
                    label,
                    style: theme.typography.body.sm.copyWith(
                      color: theme.colors.foreground,
                      fontWeight: selected ? FontWeight.w600 : FontWeight.w500,
                    ),
                  ),
                ),
                const SizedBox(width: 12),
                SizedBox(
                  width: 64,
                  height: 28,
                  child: CustomPaint(
                    painter: _CurvePreviewPainter(
                      channels: channels,
                      colors: colors,
                    ),
                  ),
                ),
              ],
            ),
          ),
        );
      },
    );
  }
}

class _RadioMark extends StatelessWidget {
  const new({required this.selected});

  final bool selected;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return Container(
      width: 16,
      height: 16,
      decoration: BoxDecoration(
        shape: BoxShape.circle,
        border: Border.all(
          color: selected ? theme.colors.primary : theme.colors.border,
          width: 1.5,
        ),
      ),
      child: Center(
        child: selected
            ? Container(
                width: 8,
                height: 8,
                decoration: BoxDecoration(
                  shape: BoxShape.circle,
                  color: theme.colors.primary,
                ),
              )
            : null,
      ),
    );
  }
}

class _CurvePreviewPainter extends CustomPainter {
  const new({required this.channels, required this.colors});

  final List<List<double>> channels;
  final List<Color> colors;

  @override
  void paint(Canvas canvas, Size size) {
    if (channels.isEmpty || size.isEmpty) {
      return;
    }
    const inset = 1.5;
    final width = size.width - inset * 2;
    final height = size.height - inset * 2;

    for (var channel = 0; channel < channels.length; channel++) {
      final values = channels[channel];
      if (values.length < 2) {
        continue;
      }
      final paint = Paint()
        ..color = colors[channel % colors.length]
        ..style = PaintingStyle.stroke
        ..strokeWidth = 2
        ..strokeCap = StrokeCap.round
        ..strokeJoin = StrokeJoin.round
        ..isAntiAlias = true;
      final path = Path();
      for (var index = 0; index < values.length; index++) {
        final x = inset + width * index / (values.length - 1);
        final y = inset + height * (1 - values[index].clamp(0.0, 1.0));
        if (index == 0) {
          path.moveTo(x, y);
        } else {
          path.lineTo(x, y);
        }
      }
      canvas.drawPath(path, paint);
    }
  }

  @override
  bool shouldRepaint(covariant _CurvePreviewPainter oldDelegate) {
    return oldDelegate.channels != channels || oldDelegate.colors != colors;
  }
}

/// Pill verdict: verified (green), blocked (red), or a neutral hint.
class _CapabilityBadge extends StatelessWidget {
  const new({required this.status});

  final _CapabilityStatus status;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: status.fill,
        border: Border.all(color: status.border),
        borderRadius: theme.style.borderRadius.pill,
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          spacing: 6,
          children: [
            Icon(status.icon, size: 14, color: status.color),
            Flexible(
              child: Text(
                status.text,
                style: theme.typography.body.sm.copyWith(color: status.color),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
