import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/mixer/fader_slider.dart';
import 'package:gui_flutter/mixer/level_meter.dart';
import 'package:gui_flutter/mixer/rotary_knob.dart';
import 'package:gui_flutter/src/rust/api/engine.dart';

/// Matches Forui `FButton(size: .sm)` desktop height used for cue / meter spacer.
const _columnFooterHeight = 32.0;

/// Tick half-span (gap + major) ≈ 15; thumb 20 — keep fader ≥ this.
const _faderMinHitWidth = 36.0;

/// Center mixer: Tauri `DeckMixer` layout wired to [EngineTransport].
class MixerStrip extends ConsumerStatefulWidget {
  const MixerStrip({super.key});

  @override
  ConsumerState<MixerStrip> createState() => _MixerStripState();
}

class _MixerStripState extends ConsumerState<MixerStrip> {
  var _meterMono = true;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final enabled = ref.watch(engineRunningProvider);
    final crossfader = ref.watch(crossfaderProvider) * 100;

    return SizedBox(
      width: 232,
      child: FCard(
        child: Padding(
          padding: const EdgeInsets.all(10),
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
                  spacing: 8,
                  children: [
                    _EqColumn(deckId: 0, accent: .a, enabled: enabled),
                    _VolumeColumn(deckId: 0, accent: .a, enabled: enabled),
                    _LevelMetersColumn(
                      mono: _meterMono,
                      onMonoChanged: (mono) =>
                          setState(() => _meterMono = mono),
                    ),
                    _VolumeColumn(deckId: 1, accent: .b, enabled: enabled),
                    _EqColumn(deckId: 1, accent: .b, enabled: enabled),
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
                        value: crossfader,
                        min: 0,
                        max: 100,
                        step: 0.05,
                        showIndicator: false,
                        showMarkers: true,
                        centerNotch: true,
                        crossfaderTrack: true,
                        disabled: !enabled,
                        onValueChange: (next) {
                          unawaited(
                            _mixerCmd(
                              context,
                              () => setCrossfader(ref, next / 100),
                            ),
                          );
                        },
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

Future<void> _mixerCmd(BuildContext context, Future<void> Function() fn) async {
  try {
    await fn();
  } catch (e) {
    if (!context.mounted) {
      return;
    }
    showFToast(context: context, variant: .destructive, title: Text('$e'));
  }
}

class _EqColumn extends ConsumerWidget {
  const _EqColumn({
    required this.deckId,
    required this.accent,
    required this.enabled,
  });

  final int deckId;
  final FaderAccent accent;
  final bool enabled;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final accentColor = FaderColors.forAccent(accent).grip;
    final ch = ref.watch(deckMixerChannelProvider(deckId));

    Widget knob(
      String label,
      double value, {
      required Future<void> Function(double) send,
    }) {
      return RotaryKnob(
        label: label,
        value: value,
        min: kControlNormMin,
        max: kControlNormMax,
        step: kControlNormStep,
        center: kControlNormCenter,
        size: .md,
        accentColor: accentColor,
        disabled: !enabled,
        onValueChange: (next) {
          unawaited(_mixerCmd(context, () => send(next)));
        },
      );
    }

    return Column(
      children: [
        knob(
          'HI',
          ch.eqHigh,
          send: (v) => setDeckEqBand(ref, deckId, EqBand.high, v),
        ),
        const SizedBox(height: 4),
        knob(
          'MID',
          ch.eqMid,
          send: (v) => setDeckEqBand(ref, deckId, EqBand.mid, v),
        ),
        const SizedBox(height: 4),
        knob(
          'LOW',
          ch.eqLow,
          send: (v) => setDeckEqBand(ref, deckId, EqBand.low, v),
        ),
        const SizedBox(height: 4),
        knob('FLT', ch.filter, send: (v) => setDeckFilter(ref, deckId, v)),
      ],
    );
  }
}

class _VolumeColumn extends ConsumerWidget {
  const _VolumeColumn({
    required this.deckId,
    required this.accent,
    required this.enabled,
  });

  final int deckId;
  final FaderAccent accent;
  final bool enabled;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final accentColor = FaderColors.forAccent(accent).grip;
    final ch = ref.watch(deckMixerChannelProvider(deckId));

    return Column(
      children: [
        _MixerGainHeader(
          gain: ch.gainTrim,
          accentColor: accentColor,
          disabled: !enabled,
          onGain: (next) {
            unawaited(
              _mixerCmd(context, () => setDeckGainTrim(ref, deckId, next)),
            );
          },
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
                  value: ch.volume * 100,
                  min: 0,
                  max: 100,
                  showIndicator: true,
                  showMarkers: true,
                  disabled: !enabled,
                  onValueChange: (next) {
                    unawaited(
                      _mixerCmd(
                        context,
                        () => setDeckVolume(ref, deckId, next / 100),
                      ),
                    );
                  },
                ),
              ),
            ),
          ),
        ),
        _MixerCueFooter(
          cue: ch.headphoneCue,
          disabled: !enabled,
          onCue: () {
            unawaited(
              _mixerCmd(
                context,
                () => setDeckHeadphoneCue(ref, deckId, !ch.headphoneCue),
              ),
            );
          },
        ),
      ],
    );
  }
}

/// Shared GAIN block so meters and volume columns stay vertically aligned.
class _MixerGainHeader extends StatelessWidget {
  const _MixerGainHeader({
    required this.gain,
    required this.accentColor,
    required this.onGain,
    required this.disabled,
  });

  final double gain;
  final Color accentColor;
  final ValueChanged<double> onGain;
  final bool disabled;

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
      disabled: disabled,
      onValueChange: onGain,
    );
  }
}

class _MixerCueFooter extends StatelessWidget {
  const _MixerCueFooter({
    required this.cue,
    required this.onCue,
    this.disabled = false,
    this.spacer = false,
  });

  final bool cue;
  final VoidCallback onCue;
  final bool disabled;
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
          onPress: disabled ? null : onCue,
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
class _LevelMetersColumn extends ConsumerWidget {
  const _LevelMetersColumn({required this.mono, required this.onMonoChanged});

  final bool mono;
  final ValueChanged<bool> onMonoChanged;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = context.theme;
    final mode = mono ? LevelMeterMode.mono : LevelMeterMode.stereo;
    final levelsA = ref.watch(deckLevelsProvider(0));
    final levelsB = ref.watch(deckLevelsProvider(1));

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
                LevelMeter(levels: levelsA, mode: mode),
                const SizedBox(width: 2),
                LevelMeter(levels: levelsB, mode: mode),
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
