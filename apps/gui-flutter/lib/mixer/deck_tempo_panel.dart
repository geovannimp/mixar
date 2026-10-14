import 'package:flutter/widgets.dart';
import 'package:gui_flutter/mixer/fader_slider.dart';
import 'package:gui_flutter/mixer/mixer_button.dart';
import 'package:gui_flutter/mixer/mixer_live_color.dart';
import 'package:gui_flutter/mixer/pad_modes.dart';
import 'package:gui_flutter/mixer/tempo_format.dart';
import 'package:gui_flutter/shell/app_tooltip.dart';
import 'package:gui_flutter/shell/app_typography.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:gui_flutter/src/rust/api/engine.dart' show SyncMode;
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:skeletonizer/skeletonizer.dart';

/// Tauri-shaped tempo column: BPM / pitch / sync + pitch fader from engine state.
class DeckTempoPanel extends StatelessWidget {
  const new({
    required this.accent,
    required this.speed,
    required this.tempoRange,
    required this.syncMode,
    required this.isMaster,
    required this.onSpeedChange,
    required this.onTempoRangeChange,
    required this.onToggleSync,
    this.trackBpm,
    this.loading = false,
    this.tempoRangeSteps = kTempoRangeSteps,
    this.enabled = true,
    super.key,
  });

  final FaderAccent accent;
  final double speed;
  final double tempoRange;
  final SyncMode syncMode;
  final bool isMaster;
  final ValueChanged<double> onSpeedChange;
  final ValueChanged<double> onTempoRangeChange;
  final ValueChanged<bool> onToggleSync;

  /// Original track BPM when loaded; null → `—` and no live BPM scaling display source.
  final double? trackBpm;

  /// Skeletonize the BPM readout while a track is loading.
  final bool loading;

  final List<double> tempoRangeSteps;

  /// When false (engine not running), disable tempo controls.
  final bool enabled;

  bool get _syncActive => syncMode != SyncMode.off;

  ({String tip, String description}) get _syncTooltip {
    if (isMaster) {
      return (
        tip: 'Sync master',
        description: 'This deck is the tempo reference for synced decks.',
      );
    }
    return switch (syncMode) {
      SyncMode.off => (
        tip: 'Sync',
        description:
            "Match this deck's tempo to the master. Shift+click for beat sync.",
      ),
      SyncMode.tempo => (
        tip: 'Tempo sync',
        description: 'Tempo follows the master. Click again to turn sync off.',
      ),
      SyncMode.beat => (
        tip: 'Beat sync',
        description:
            'Tempo and phase follow the master. Click again to turn sync off.',
      ),
    };
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final accent = FaderColors.forAccent(this.accent).grip;
    final liveBpm = effectiveBpm(trackBpm, speed, tempoRange);
    final sliderValue = speedToPitchSlider(speed);
    final faderDisabled = _syncActive;
    final syncOn = _syncActive && !loading;
    final syncPress = (loading || !enabled || isMaster)
        ? null
        : () => onToggleSync(shiftKeyPressed());
    final (:tip, :description) = _syncTooltip;
    // Master: crown. Synced follower: closed lock. Else open lock.
    // Colors stay the same as the lock chrome (no gold override).
    final statusIcon = isMaster
        ? LucideIcons.crown
        : (syncOn ? LucideIcons.lock : LucideIcons.lockOpen);
    final statusColor = syncOn
        ? kMixerLiveOn
        : theme.colors.mutedForeground;

    final cardRadius = theme.style.borderRadius.md;
    return SizedBox(
      width: 88,
      child: DecoratedBox(
        decoration: BoxDecoration(
          border: Border.all(color: theme.colors.border),
          borderRadius: cardRadius,
          color: theme.colors.background.withValues(alpha: 0.8),
        ),
        child: ClipRRect(
          borderRadius: cardRadius,
          child: Column(
            children: [
              // Flush to card edges; divider only under BPM/pitch/status.
              Skeletonizer(
                enabled: loading,
                child: AppTooltip(
                  tip: tip,
                  description: description,
                  child: DecoratedBox(
                    decoration: BoxDecoration(
                      border: Border(
                        bottom: BorderSide(color: theme.colors.border),
                      ),
                      color: syncOn ? theme.colors.primaryTint : null,
                    ),
                    child: MixerButton(
                      variant: .ghost,
                      size: .sm,
                      selected: syncOn,
                      padding: const EdgeInsets.symmetric(
                        horizontal: 4,
                        vertical: 8,
                      ),
                      onPress: syncPress,
                      semanticsLabel: tip,
                      child: Column(
                        mainAxisSize: .min,
                        children: [
                          Text(
                            loading ? '000.00' : formatBpm(liveBpm),
                            textAlign: .center,
                            style: theme.typography.body.md.copyWith(
                              fontFamily: MixarFonts.notoSansMono,
                              color: accent,
                              fontWeight: .w700,
                              height: 1.1,
                              fontFeatures: const [
                                FontFeature.tabularFigures(),
                                FontFeature.slashedZero(),
                              ],
                            ),
                          ),
                          const SizedBox(height: 2),
                          Text(
                            formatPitchPercent(speed, tempoRange),
                            textAlign: .center,
                            style: theme.typography.body.xs.copyWith(
                              fontFamily: MixarFonts.notoSansMono,
                              color: theme.colors.mutedForeground,
                              fontWeight: .w500,
                              fontFeatures: const [
                                FontFeature.tabularFigures(),
                              ],
                            ),
                          ),
                          const SizedBox(height: 4),
                          Icon(statusIcon, size: 14, color: statusColor),
                        ],
                      ),
                    ),
                  ),
                ),
              ),
              Expanded(
                child: Padding(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 8,
                    vertical: 12,
                  ),
                  child: FaderSlider(
                    accent: this.accent,
                    value: sliderValue,
                    step: 0.05,
                    showIndicator: false,
                    showMarkers: true,
                    centerNotch: true,
                    disabled: faderDisabled || !enabled,
                    semanticLabel: 'Tempo',
                    onValueChange: (next) =>
                        onSpeedChange(pitchSliderToSpeed(next)),
                  ),
                ),
              ),
              Padding(
                padding: const EdgeInsets.fromLTRB(8, 0, 8, 8),
                child: SizedBox(
                  width: double.infinity,
                  child: AppTooltip(
                    tip: 'Tempo range',
                    description:
                        'Pitch fader span. Click to cycle the max ±% range.',
                    child: MixerButton(
                      variant: .ghost,
                      size: .xs,
                      mainAxisSize: .min,
                      onPress: enabled
                          ? () => onTempoRangeChange(
                              nextTempoRange(tempoRange, tempoRangeSteps),
                            )
                          : null,
                      semanticsLabel: 'Tempo range',
                      child: Text(
                        formatTempoRange(tempoRange),
                        style: theme.typography.body.xs.copyWith(
                          fontSize: 12,
                          fontWeight: .w600,
                          color: theme.colors.mutedForeground,
                        ),
                      ),
                    ),
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
