import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/mixer/collapsible_section.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/mixer/fader_slider.dart';
import 'package:gui_flutter/mixer/level_meter.dart';
import 'package:gui_flutter/mixer/mixer_button.dart';
import 'package:gui_flutter/mixer/mixer_live_color.dart';
import 'package:gui_flutter/mixer/rotary_knob.dart';
import 'package:gui_flutter/settings/settings_providers.dart';
import 'package:gui_flutter/shell/app_tooltip.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:gui_flutter/shell/mixar_toast.dart';
import 'package:gui_flutter/src/rust/api/engine.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Matches Forui `MixerButton(size: .sm)` desktop height used for cue / meter spacer.
const _columnFooterHeight = 32.0;

/// Tick half-span (gap + major) ≈ 15; thumb 20 — keep fader ≥ this.
const _faderMinHitWidth = 36.0;

/// Open mixer width without center VU column (4×36 + 3×8).
const _mixerInnerWidthNoMeters = 168.0;

/// Open mixer width with center VU column (5 columns + 4×8).
const _mixerInnerWidthWithMeters = 212.0;

/// Matches [RotaryKnob] md column: label (8) + gap (2) + dial (36).
const _gainHeaderHeight = 8.0 + 2.0 + 36.0;

/// Center mixer: Tauri `DeckMixer` layout wired to [EngineTransport].
class MixerStrip extends ConsumerStatefulWidget {
  const new({super.key});

  @override
  ConsumerState<MixerStrip> createState() => _MixerStripState();
}

class _MixerStripState extends ConsumerState<MixerStrip> {
  var _mixerOpen = true;
  var _crossfaderOpen = true;

  bool get _anyOpen => _mixerOpen || _crossfaderOpen;

  @override
  Widget build(BuildContext context) {
    final showMeters = ref
        .watch(appSettingsProvider)
        .maybeWhen(data: (s) => s.showMixerMeters, orElse: () => true);
    final innerWidth = showMeters
        ? _mixerInnerWidthWithMeters
        : _mixerInnerWidthNoMeters;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: SizedBox(
        width: _anyOpen ? innerWidth : null,
        child: Column(
          children: [
            CollapsibleSection(
              label: 'Mixer',
              fillRemaining: true,
              expandHeader: _anyOpen,
              onOpenChanged: (open) => setState(() => _mixerOpen = open),
              child: const _MixerChannels(),
            ),
            CollapsibleSection(
              label: 'Crossfader',
              expandHeader: _anyOpen,
              onOpenChanged: (open) => setState(() => _crossfaderOpen = open),
              child: const Padding(
                padding: EdgeInsets.only(bottom: 8),
                child: _CrossfaderStrip(),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _MixerChannels extends ConsumerWidget {
  const new();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final enabled = ref.watch(engineRunningProvider);
    final showMeters = ref
        .watch(appSettingsProvider)
        .maybeWhen(data: (s) => s.showMixerMeters, orElse: () => true);
    final mono = ref
        .watch(appSettingsProvider)
        .maybeWhen(data: (s) => s.mixerMetersMono, orElse: () => true);
    return Row(
      mainAxisAlignment: .center,
      crossAxisAlignment: .stretch,
      spacing: 8,
      children: [
        _EqColumn(deckId: 0, accent: .a, enabled: enabled),
        _VolumeColumn(deckId: 0, accent: .a, enabled: enabled),
        if (showMeters) _LevelMetersColumn(mono: mono),
        _VolumeColumn(deckId: 1, accent: .b, enabled: enabled),
        _EqColumn(deckId: 1, accent: .b, enabled: enabled),
      ],
    );
  }
}

class _CrossfaderStrip extends ConsumerWidget {
  const new();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = context.theme;
    final enabled = ref.watch(engineRunningProvider);
    final crossfader = ref.watch(crossfaderProvider) * 100;
    return SizedBox(
      height: 24,
      child: Row(
        spacing: 10,
        children: [
          Text(
            'A',
            style: theme.typography.body.xs.copyWith(
              color: FaderColors.a.grip,
              fontWeight: .w600,
              fontSize: 8,
            ),
          ),
          Expanded(
            child: FaderSlider(
              orientation: .horizontal,
              value: crossfader,
              step: 0.05,
              showIndicator: false,
              showMarkers: true,
              centerNotch: true,
              crossfaderTrack: true,
              disabled: !enabled,
              onValueChange: (next) {
                unawaited(
                  _mixerCmd(context, () => setCrossfader(ref, next / 100)),
                );
              },
            ),
          ),
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
    showMixarToast(
      context: context,
      variant: MixarToastVariant.destructive,
      title: Text('$e'),
    );
  }
}

class _EqColumn extends ConsumerWidget {
  const new({
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
        center: kControlNormCenter,
        accentColor: accentColor,
        disabled: !enabled,
        onValueChange: (next) {
          unawaited(_mixerCmd(context, () => send(next)));
        },
      );
    }

    return FittedBox(
      fit: .scaleDown,
      alignment: .topCenter,
      child: Column(
        mainAxisSize: .min,
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
      ),
    );
  }
}

class _VolumeColumn extends ConsumerWidget {
  const new({
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
                  accent: accent,
                  value: ch.volume * 100,
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
  const new({
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
      center: kControlNormCenter,
      accentColor: accentColor,
      disabled: disabled,
      onValueChange: onGain,
    );
  }
}

class _MixerCueFooter extends StatelessWidget {
  const new({
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
    return SizedBox(
      height: _columnFooterHeight,
      child: Center(
        child: AppTooltip(
          tip: 'Headphone cue',
          child: MixerButton(
            variant: cue ? .secondary : .ghost,
            size: .sm,
            mainAxisSize: .min,
            selected: cue,
            semanticsLabel: 'Headphone cue',
            onPress: disabled ? null : onCue,
            child: Icon(
              LucideIcons.headphones,
              size: 14,
              color: cue ? kMixerLiveOn : theme.colors.mutedForeground,
            ),
          ),
        ),
      ),
    );
  }
}

/// VU ladders between faders. Mono/stereo comes from saved settings.
class _LevelMetersColumn extends ConsumerWidget {
  const new({required this.mono});

  final bool mono;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final mode = mono ? LevelMeterMode.mono : LevelMeterMode.stereo;

    return Column(
      children: [
        // Spacer matching the volume-column GAIN header height.
        const ExcludeSemantics(child: SizedBox(height: _gainHeaderHeight)),
        Expanded(
          child: Padding(
            padding: const EdgeInsets.symmetric(vertical: 4, horizontal: 2),
            child: Row(
              mainAxisAlignment: .center,
              crossAxisAlignment: .stretch,
              children: [
                RepaintBoundary(child: _LiveLevelMeter(deckId: 0, mode: mode)),
                const SizedBox(width: 2),
                RepaintBoundary(child: _LiveLevelMeter(deckId: 1, mode: mode)),
              ],
            ),
          ),
        ),
        _MixerCueFooter(cue: false, onCue: () {}, spacer: true),
      ],
    );
  }
}

class _LiveLevelMeter extends ConsumerWidget {
  const new({required this.deckId, required this.mode});

  final int deckId;
  final LevelMeterMode mode;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final levels = ref.watch(deckLevelsProvider(deckId));
    return LevelMeter(levels: levels, mode: mode);
  }
}
