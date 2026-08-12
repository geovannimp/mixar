import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/fader_slider.dart';
import 'package:gui_flutter/mixer/level_meter.dart';
import 'package:gui_flutter/mixer/rotary_knob.dart';

const _eqColumnWidth = 52.0;
const _faderColumnWidth = 52.0;

/// Matches Forui `FButton(size: .sm)` desktop height used for cue / meter spacer.
const _columnFooterHeight = 32.0;

/// Tick half-span (gap + major) ≈ 15; thumb 20 — keep fader ≥ this.
const _faderMinHitWidth = 36.0;

/// Center mixer: Tauri `DeckMixer` layout (local state; no engine).
class MixerStrip extends StatefulWidget {
  const MixerStrip({super.key});

  @override
  State<MixerStrip> createState() => _MixerStripState();
}

class _MixerStripState extends State<MixerStrip> {
  double _crossfader = 50;
  bool _meterMono = true;

  final _a = _ChannelState();
  final _b = _ChannelState();

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;

    return SizedBox(
      width: 300,
      child: FCard(
        child: Padding(
          padding: const EdgeInsets.fromLTRB(10, 10, 10, 10),
          child: Column(
            children: [
              Text(
                'Mixer',
                style: theme.typography.body.xs.copyWith(
                  color: theme.colors.mutedForeground,
                  fontWeight: .w600,
                  letterSpacing: 1.6,
                ),
              ),
              const SizedBox(height: 8),
              Expanded(
                child: Row(
                  mainAxisAlignment: .center,
                  crossAxisAlignment: .stretch,
                  children: [
                    _EqColumn(
                      accent: .a,
                      high: _a.high,
                      mid: _a.mid,
                      low: _a.low,
                      filter: _a.filter,
                      onHigh: (v) => setState(() => _a.high = v),
                      onMid: (v) => setState(() => _a.mid = v),
                      onLow: (v) => setState(() => _a.low = v),
                      onFilter: (v) => setState(() => _a.filter = v),
                    ),
                    const SizedBox(width: 4),
                    _VolumeColumn(
                      accent: .a,
                      gain: _a.gain,
                      volume: _a.volume,
                      cue: _a.cue,
                      onGain: (v) => setState(() => _a.gain = v),
                      onVolume: (v) => setState(() => _a.volume = v),
                      onCue: () => setState(() => _a.cue = !_a.cue),
                    ),
                    const SizedBox(width: 4),
                    _LevelMetersColumn(
                      mono: _meterMono,
                      onMonoChanged: (mono) =>
                          setState(() => _meterMono = mono),
                    ),
                    const SizedBox(width: 4),
                    _VolumeColumn(
                      accent: .b,
                      gain: _b.gain,
                      volume: _b.volume,
                      cue: _b.cue,
                      onGain: (v) => setState(() => _b.gain = v),
                      onVolume: (v) => setState(() => _b.volume = v),
                      onCue: () => setState(() => _b.cue = !_b.cue),
                    ),
                    const SizedBox(width: 4),
                    _EqColumn(
                      accent: .b,
                      high: _b.high,
                      mid: _b.mid,
                      low: _b.low,
                      filter: _b.filter,
                      onHigh: (v) => setState(() => _b.high = v),
                      onMid: (v) => setState(() => _b.mid = v),
                      onLow: (v) => setState(() => _b.low = v),
                      onFilter: (v) => setState(() => _b.filter = v),
                    ),
                  ],
                ),
              ),
              const FDivider(
                style: .delta(padding: .value(.symmetric(vertical: 8))),
              ),
              Text(
                'Crossfader',
                style: theme.typography.body.xs.copyWith(
                  color: theme.colors.mutedForeground,
                  fontWeight: .w600,
                  letterSpacing: 1.2,
                  fontSize: 8,
                ),
              ),
              const SizedBox(height: 4),
              SizedBox(
                height: 36,
                child: Row(
                  children: [
                    Text(
                      'A',
                      style: theme.typography.body.xs.copyWith(
                        color: FaderColors.a.grip,
                        fontWeight: .w600,
                        fontSize: 8,
                      ),
                    ),
                    const SizedBox(width: 6),
                    Expanded(
                      child: FaderSlider(
                        orientation: .horizontal,
                        value: _crossfader,
                        min: 0,
                        max: 100,
                        step: 0.05,
                        showIndicator: false,
                        showMarkers: true,
                        centerNotch: true,
                        crossfaderTrack: true,
                        onValueChange: (next) =>
                            setState(() => _crossfader = next),
                      ),
                    ),
                    const SizedBox(width: 6),
                    Text(
                      'B',
                      style: theme.typography.body.xs.copyWith(
                        color: FaderColors.b.grip,
                        fontWeight: .w600,
                        fontSize: 8,
                      ),
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _ChannelState {
  double gain = kControlNormCenter;
  double high = kControlNormCenter;
  double mid = kControlNormCenter;
  double low = kControlNormCenter;
  double filter = kControlNormCenter;
  double volume = 100;
  bool cue = false;
}

class _EqColumn extends StatelessWidget {
  const _EqColumn({
    required this.accent,
    required this.high,
    required this.mid,
    required this.low,
    required this.filter,
    required this.onHigh,
    required this.onMid,
    required this.onLow,
    required this.onFilter,
  });

  final FaderAccent accent;
  final double high;
  final double mid;
  final double low;
  final double filter;
  final ValueChanged<double> onHigh;
  final ValueChanged<double> onMid;
  final ValueChanged<double> onLow;
  final ValueChanged<double> onFilter;

  @override
  Widget build(BuildContext context) {
    final accentColor = FaderColors.forAccent(accent).grip;

    Widget knob(String label, double value, ValueChanged<double> onChange) {
      return RotaryKnob(
        label: label,
        value: value,
        min: kControlNormMin,
        max: kControlNormMax,
        step: kControlNormStep,
        center: kControlNormCenter,
        size: .md,
        accentColor: accentColor,
        onValueChange: onChange,
      );
    }

    return SizedBox(
      width: _eqColumnWidth,
      child: Column(
        children: [
          knob('HI', high, onHigh),
          const SizedBox(height: 4),
          knob('MID', mid, onMid),
          const SizedBox(height: 4),
          knob('LOW', low, onLow),
          const SizedBox(height: 4),
          knob('FLT', filter, onFilter),
        ],
      ),
    );
  }
}

class _VolumeColumn extends StatelessWidget {
  const _VolumeColumn({
    required this.accent,
    required this.gain,
    required this.volume,
    required this.cue,
    required this.onGain,
    required this.onVolume,
    required this.onCue,
  });

  final FaderAccent accent;
  final double gain;
  final double volume;
  final bool cue;
  final ValueChanged<double> onGain;
  final ValueChanged<double> onVolume;
  final VoidCallback onCue;

  @override
  Widget build(BuildContext context) {
    final accentColor = FaderColors.forAccent(accent).grip;

    return SizedBox(
      width: _faderColumnWidth,
      child: Column(
        children: [
          _MixerGainHeader(
            gain: gain,
            accentColor: accentColor,
            onGain: onGain,
          ),
          Expanded(
            child: Padding(
              padding: const EdgeInsets.symmetric(vertical: 12),
              child: Center(
                child: SizedBox(
                  width: _faderMinHitWidth,
                  child: FaderSlider(
                    orientation: .vertical,
                    accent: accent,
                    value: volume,
                    min: 0,
                    max: 100,
                    showIndicator: true,
                    showMarkers: true,
                    onValueChange: onVolume,
                  ),
                ),
              ),
            ),
          ),
          _MixerCueFooter(cue: cue, onCue: onCue),
        ],
      ),
    );
  }
}

/// Shared GAIN block so meters and volume columns stay vertically aligned.
class _MixerGainHeader extends StatelessWidget {
  const _MixerGainHeader({
    required this.gain,
    required this.accentColor,
    required this.onGain,
  });

  final double gain;
  final Color accentColor;
  final ValueChanged<double> onGain;

  @override
  Widget build(BuildContext context) {
    return RotaryKnob(
      label: 'GAIN',
      value: gain,
      min: kControlNormMin,
      max: kControlNormMax,
      step: kControlNormStep,
      center: kControlNormCenter,
      size: .md,
      accentColor: accentColor,
      onValueChange: onGain,
    );
  }
}

class _MixerCueFooter extends StatelessWidget {
  const _MixerCueFooter({
    required this.cue,
    required this.onCue,
    this.spacer = false,
  });

  final bool cue;
  final VoidCallback onCue;
  final bool spacer;

  @override
  Widget build(BuildContext context) {
    if (spacer) {
      return const SizedBox(height: _columnFooterHeight);
    }
    final theme = context.theme;
    const cueOn = Color(0xbf34d399); // emerald-400 @ ~75%
    return SizedBox(
      height: _columnFooterHeight,
      child: Center(
        child: FButton(
          variant: cue ? .secondary : .ghost,
          size: .sm,
          mainAxisSize: .min,
          onPress: onCue,
          child: Icon(
            FLucideIcons.headphones,
            size: 14,
            color: cue ? cueOn : theme.colors.mutedForeground,
          ),
        ),
      ),
    );
  }
}

/// VU ladders between faders — M/S toggle sits in the GAIN-aligned header.
class _LevelMetersColumn extends StatelessWidget {
  const _LevelMetersColumn({
    required this.mono,
    required this.onMonoChanged,
  });

  final bool mono;
  final ValueChanged<bool> onMonoChanged;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final mode = mono ? LevelMeterMode.mono : LevelMeterMode.stereo;

    return Column(
      children: [
        // Invisible GAIN knob keeps meter track aligned with volume faders;
        // M/S sits centered on that header slot.
        Stack(
          alignment: Alignment.center,
          children: [
            ExcludeSemantics(
              child: IgnorePointer(
                child: Opacity(
                  opacity: 0,
                  child: RotaryKnob(
                    label: 'GAIN',
                    value: kControlNormCenter,
                    min: kControlNormMin,
                    max: kControlNormMax,
                    step: kControlNormStep,
                    center: kControlNormCenter,
                    size: .md,
                    onValueChange: _noopGain,
                  ),
                ),
              ),
            ),
            FButton(
              variant: .outline,
              size: .xs,
              mainAxisSize: .min,
              selected: mono,
              semanticsLabel: mono
                  ? 'Level meters: mono. Switch to stereo.'
                  : 'Level meters: stereo. Switch to mono.',
              onPress: () => onMonoChanged(!mono),
              child: Text(
                mono ? 'M' : 'S',
                style: theme.typography.body.xs.copyWith(
                  fontSize: 8,
                  fontWeight: .w600,
                  color: theme.colors.mutedForeground,
                ),
              ),
            ),
          ],
        ),
        Expanded(
          child: Padding(
            padding: const EdgeInsets.symmetric(vertical: 4, horizontal: 2),
            child: Row(
              mainAxisAlignment: .center,
              crossAxisAlignment: .stretch,
              children: [
                LevelMeter(levels: zeroDeckLevels, mode: mode),
                const SizedBox(width: 2),
                LevelMeter(levels: zeroDeckLevels, mode: mode),
              ],
            ),
          ),
        ),
        _MixerCueFooter(cue: false, onCue: () {}, spacer: true),
      ],
    );
  }
}

void _noopGain(double _) {}
