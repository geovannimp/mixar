import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/fader_slider.dart';
import 'package:gui_flutter/mixer/tempo_format.dart';

enum TempoSyncMode { off, tempo, beat }

/// Tauri-shaped tempo column: BPM / pitch / sync stubs + pitch fader (local state).
class DeckTempoPanel extends StatefulWidget {
  const DeckTempoPanel({
    required this.accent,
    this.trackBpm,
    super.key,
  });

  final FaderAccent accent;

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
  bool _isMaster = false;

  bool get _syncActive => _sync != TempoSyncMode.off;
  bool get _faderDisabled => _syncActive;

  void _toggleSync() {
    if (_isMaster) {
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
                  variant: _syncActive || _isMaster ? .secondary : .ghost,
                  size: .sm,
                  onPress: _isMaster ? null : _toggleSync,
                  child: Text(
                    _isMaster
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
              if (_isMaster)
                SizedBox(
                  width: double.infinity,
                  child: Text(
                    'Master',
                    textAlign: .center,
                    style: theme.typography.body.xs.copyWith(
                      color: const Color(0xe634d399), // emerald-400
                      fontWeight: .w600,
                      letterSpacing: 0.6,
                      fontSize: 9,
                    ),
                  ),
                )
              else
                GestureDetector(
                  behavior: HitTestBehavior.opaque,
                  onTap: () => setState(() {
                    _isMaster = true;
                    _sync = TempoSyncMode.off;
                  }),
                  child: SizedBox(
                    width: double.infinity,
                    child: Text(
                      'Set master',
                      textAlign: .center,
                      style: theme.typography.body.xs.copyWith(
                        color: theme.colors.mutedForeground,
                        fontWeight: .w500,
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
