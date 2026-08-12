import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/fader_slider.dart';
import 'package:gui_flutter/mixer/tempo_format.dart';

enum TempoSyncMode { off, tempo, beat }

/// Tauri-shaped tempo column: BPM / pitch / sync stubs + pitch fader (local state).
class DeckTempoPanel extends StatefulWidget {
  const DeckTempoPanel({
    required this.accent,
    required this.isMaster,
    required this.onMasterChanged,
    this.trackBpm,
    super.key,
  });

  final FaderAccent accent;
  final bool isMaster;
  final ValueChanged<bool> onMasterChanged;

  /// Original track BPM when loaded; null → `—` and no live BPM scaling display source.
  final double? trackBpm;

  @override
  State<DeckTempoPanel> createState() => _DeckTempoPanelState();
}

class _DeckTempoPanelState extends State<DeckTempoPanel> {
  /// Tempo fader position `0..1` (mid = unity). Slider UI is 0–100.
  double _speedNorm = 0.5;
  double _tempoRange = kDefaultTempoRange;
  TempoSyncMode _sync = TempoSyncMode.off;

  bool get _syncActive => _sync != TempoSyncMode.off;
  bool get _faderDisabled => _syncActive;

  @override
  void didUpdateWidget(covariant DeckTempoPanel oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.isMaster && !oldWidget.isMaster) {
      _sync = TempoSyncMode.off;
    }
  }

  void _toggleSync() {
    if (widget.isMaster) {
      return;
    }
    setState(() {
      _sync = switch (_sync) {
        TempoSyncMode.off => TempoSyncMode.tempo,
        TempoSyncMode.tempo => TempoSyncMode.beat,
        TempoSyncMode.beat => TempoSyncMode.off,
      };
    });
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final accent = FaderColors.forAccent(widget.accent).grip;
    final liveBpm = effectiveBpm(widget.trackBpm, _speedNorm, _tempoRange);
    final sliderValue = speedToPitchSlider(_speedNorm);
    final isMaster = widget.isMaster;

    final chipStyle = theme.typography.body.xs.copyWith(
      fontWeight: .w600,
      fontSize: 10,
    );

    return SizedBox(
      width: 80,
      child: DecoratedBox(
        decoration: BoxDecoration(
          border: Border.all(color: theme.colors.border),
          borderRadius: BorderRadius.circular(6),
          color: theme.colors.background.withValues(alpha: 0.8),
        ),
        child: Padding(
          padding: const EdgeInsets.fromLTRB(6, 8, 6, 6),
          child: Column(
            children: [
              Text(
                formatBpm(liveBpm),
                textAlign: .center,
                style: theme.typography.body.sm.copyWith(
                  color: accent,
                  fontWeight: .w700,
                  fontFeatures: const [FontFeature.tabularFigures()],
                ),
              ),
              const SizedBox(height: 2),
              Text(
                formatPitchPercent(_speedNorm, _tempoRange),
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
                  variant: _syncActive || isMaster ? .secondary : .ghost,
                  size: .sm,
                  onPress: isMaster ? null : _toggleSync,
                  child: Text(
                    isMaster
                        ? 'M'
                        : switch (_sync) {
                            TempoSyncMode.off => 'Sync',
                            TempoSyncMode.tempo => 'S',
                            TempoSyncMode.beat => 'B',
                          },
                    style: chipStyle,
                  ),
                ),
              ),
              const SizedBox(height: 4),
              SizedBox(
                width: double.infinity,
                child: FButton(
                  variant: isMaster ? .secondary : .ghost,
                  size: .sm,
                  onPress: () {
                    if (isMaster) {
                      widget.onMasterChanged(false);
                    } else {
                      setState(() => _sync = TempoSyncMode.off);
                      widget.onMasterChanged(true);
                    }
                  },
                  child: Text(
                    isMaster ? 'Master' : 'Set master',
                    style: theme.typography.body.xs.copyWith(
                      color: isMaster
                          ? const Color(0xe634d399) // emerald-400
                          : theme.colors.mutedForeground,
                      fontWeight: .w600,
                      letterSpacing: 0.4,
                      fontSize: 9,
                    ),
                  ),
                ),
              ),
              const SizedBox(height: 14),
              Expanded(
                child: Padding(
                  padding: const EdgeInsets.symmetric(horizontal: 12),
                  child: FaderSlider(
                    orientation: .vertical,
                    accent: widget.accent,
                    value: sliderValue,
                    min: 0,
                    max: 100,
                    step: 0.05,
                    showIndicator: false,
                    showMarkers: true,
                    centerNotch: true,
                    disabled: _faderDisabled,
                    semanticLabel: 'Tempo',
                    onValueChange: (next) => setState(() {
                      _speedNorm = pitchSliderToSpeed(next);
                    }),
                  ),
                ),
              ),
              const SizedBox(height: 14),
              SizedBox(
                width: double.infinity,
                child: FButton(
                  variant: .secondary,
                  size: .sm,
                  onPress: () => setState(() {
                    _tempoRange = nextTempoRange(_tempoRange);
                  }),
                  child: Text(formatTempoRange(_tempoRange), style: chipStyle),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
