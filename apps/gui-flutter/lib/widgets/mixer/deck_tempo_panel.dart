import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/models/pad_modes.dart';
import 'package:gui_flutter/core/theme/app_typography.dart';
import 'package:gui_flutter/core/utils/tempo_format.dart';
import 'package:gui_flutter/src/rust/api/engine.dart' show SyncMode;
import 'package:gui_flutter/widgets/mixer/fader_slider.dart';
import 'package:gui_flutter/widgets/mixer/mixer_button.dart';
import 'package:gui_flutter/widgets/shell/app_tooltip.dart';
import 'package:gui_flutter/widgets/shell/m_tappable.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:skeletonizer/skeletonizer.dart';

/// Full-height tempo rail: BPM / pitch / sync header, pitch fader, range chip.
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
        ? theme.colors.primary
        : theme.colors.mutedForeground;

    return SizedBox(
      width: 84,
      child: DecoratedBox(
        // Side rails only: the column is full-bleed from the waveform pane down
        // to the library divider, so there is no card chrome. Foreground keeps
        // the hairlines visible under the synced header tint.
        position: DecorationPosition.foreground,
        decoration: BoxDecoration(
          border: Border(
            left: BorderSide(color: theme.colors.border),
            right: BorderSide(color: theme.colors.border),
          ),
        ),
        child: Column(
          children: [
            // Divider only under BPM/pitch/status.
            // MTappable (not MixerButton): MixerButton's inner Row overflows
            // the narrow tempo rail when wrapping a tall BPM/pitch Column.
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
                  child: SizedBox(
                    width: double.infinity,
                    child: MTappable.child(
                      selected: syncOn,
                      onPress: syncPress,
                      semanticsLabel: tip,
                      child: Padding(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 4,
                          vertical: 8,
                        ),
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
    );
  }
}
