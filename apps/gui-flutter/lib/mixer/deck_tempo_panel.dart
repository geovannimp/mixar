import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/fader_slider.dart';
import 'package:gui_flutter/mixer/pad_modes.dart';
import 'package:gui_flutter/mixer/tempo_format.dart';
import 'package:gui_flutter/src/rust/api/engine.dart' show SyncMode;
import 'package:skeletonizer/skeletonizer.dart';

/// Tauri-shaped tempo column: BPM / pitch / sync + pitch fader from engine state.
class DeckTempoPanel extends StatelessWidget {
  const DeckTempoPanel({
    required this.accent,
    required this.speed,
    required this.tempoRange,
    required this.syncMode,
    required this.isMaster,
    required this.onSpeedChange,
    required this.onTempoRangeChange,
    required this.onToggleSync,
    required this.onSetMaster,
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
  final VoidCallback onSetMaster;

  /// Original track BPM when loaded; null → `—` and no live BPM scaling display source.
  final double? trackBpm;

  /// Skeletonize the BPM readout while a track is loading.
  final bool loading;

  final List<double> tempoRangeSteps;

  /// When false (no track), disable tempo controls.
  final bool enabled;

  bool get _syncActive => syncMode != SyncMode.off;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final accent = FaderColors.forAccent(this.accent).grip;
    final liveBpm = effectiveBpm(trackBpm, speed, tempoRange);
    final sliderValue = speedToPitchSlider(speed);
    final faderDisabled = _syncActive;

    final chipStyle = theme.typography.body.xs.copyWith(
      fontWeight: .w600,
      fontSize: 10,
    );
    const compactPad = EdgeInsets.symmetric(horizontal: 4, vertical: 6);

    return SizedBox(
      width: 84,
      child: DecoratedBox(
        decoration: BoxDecoration(
          border: Border.all(color: theme.colors.border),
          borderRadius: theme.style.borderRadius.md,
          color: theme.colors.background.withValues(alpha: 0.8),
        ),
        child: Padding(
          padding: const EdgeInsets.all(8),
          child: Column(
            children: [
              Skeletonizer(
                enabled: loading,
                child: Text(
                  loading ? '000.00' : formatBpm(liveBpm),
                  textAlign: .center,
                  style: theme.typography.body.sm.copyWith(
                    color: accent,
                    fontWeight: .w700,
                    fontFeatures: const [FontFeature.tabularFigures()],
                  ),
                ),
              ),
              const SizedBox(height: 2),
              Text(
                formatPitchPercent(speed, tempoRange),
                textAlign: .center,
                style: theme.typography.body.xs.copyWith(
                  color: theme.colors.mutedForeground,
                  fontWeight: .w500,
                  fontFeatures: const [FontFeature.tabularFigures()],
                ),
              ),
              const SizedBox(height: 6),
              SizedBox(
                width: double.infinity,
                child: FButton(
                  variant: .secondary,
                  size: .sm,
                  style: .delta(
                    contentStyle: .delta(padding: .value(compactPad)),
                  ),
                  onPress: (!enabled || isMaster)
                      ? null
                      : () => onToggleSync(shiftKeyPressed()),
                  child: Text(
                    isMaster
                        ? 'M'
                        : switch (syncMode) {
                            SyncMode.off => 'Sync',
                            SyncMode.tempo => 'S',
                            SyncMode.beat => 'B',
                          },
                    style: chipStyle,
                  ),
                ),
              ),
              const SizedBox(height: 2),
              SizedBox(
                width: double.infinity,
                child: FButton(
                  variant: isMaster ? .secondary : .ghost,
                  size: .xs,
                  style: .delta(
                    contentStyle: .delta(padding: .value(compactPad)),
                  ),
                  onPress: (!enabled || isMaster) ? null : onSetMaster,
                  child: SizedBox(
                    width: 52,
                    child: FittedBox(
                      fit: .scaleDown,
                      child: Text(
                        isMaster ? 'Master' : 'Set master',
                        textAlign: .center,
                        maxLines: 1,
                        style: theme.typography.body.xs.copyWith(
                          color: isMaster
                              ? const Color(0xe634d399) // emerald-400
                              : theme.colors.mutedForeground,
                          fontWeight: .w600,
                          fontSize: 9,
                          height: 1.1,
                        ),
                      ),
                    ),
                  ),
                ),
              ),
              Expanded(
                child: Padding(
                  padding: const EdgeInsets.symmetric(vertical: 12),
                  child: FaderSlider(
                    orientation: .vertical,
                    accent: this.accent,
                    value: sliderValue,
                    min: 0,
                    max: 100,
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
              SizedBox(
                width: double.infinity,
                child: FButton(
                  variant: .ghost,
                  size: .xs,
                  mainAxisSize: .min,
                  onPress: enabled
                      ? () => onTempoRangeChange(
                          nextTempoRange(tempoRange, tempoRangeSteps),
                        )
                      : null,
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
            ],
          ),
        ),
      ),
    );
  }
}
