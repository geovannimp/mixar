import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/core/models/bus_enums.dart';
import 'package:gui_flutter/src/rust/api/engine.dart';
import 'package:gui_flutter/widgets/mixer/collapsible_section.dart';
import 'package:gui_flutter/widgets/mixer/fader_slider.dart';
import 'package:gui_flutter/widgets/mixer/level_meter.dart';
import 'package:gui_flutter/widgets/mixer/mixer_button.dart';
import 'package:gui_flutter/widgets/mixer/rotary_knob.dart';
import 'package:gui_flutter/widgets/shell/app_tooltip.dart';
import 'package:gui_flutter/widgets/shell/m_tappable.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_anchor.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_item.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:gui_flutter/widgets/shell/mixar_toast.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Matches Forui `MixerButton(size: .sm)` desktop height used for cue / meter spacer.
const _columnFooterHeight = 32.0;

/// Tick half-span (gap + major) ≈ 16; grip 26 — keep fader ≥ this.
const _faderMinHitWidth = 36.0;

/// Open mixer width without center VU column (4×36 + 3×8).
const _mixerInnerWidthNoMeters = 168.0;

/// Open mixer width with center VU column (5 columns + 4×8).
const _mixerInnerWidthWithMeters = 212.0;
const _mixerInnerWidthFour = 388.0;

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
    final settings = ref.watch(appSettingsProvider).value;
    final showMeters = settings?.showMixerMeters ?? true;
    final numDecks = settings?.numDecks ?? 2;
    final innerWidth = numDecks == 4
        ? _mixerInnerWidthFour
        : showMeters
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
    final settings = ref.watch(appSettingsProvider).value;
    final showMeters = settings?.showMixerMeters ?? true;
    final mono = settings?.mixerMetersMono ?? true;
    final numDecks = settings?.numDecks ?? 2;
    if (numDecks == 2) {
      return Row(
        mainAxisAlignment: .center,
        crossAxisAlignment: .stretch,
        spacing: 8,
        children: [
          _EqColumn(deckId: 0, accent: .a, enabled: enabled),
          _VolumeColumn(deckId: 0, accent: .a, enabled: enabled),
          if (showMeters) _LevelMetersColumn(mono: mono, deckIds: const [0, 1]),
          _VolumeColumn(deckId: 1, accent: .b, enabled: enabled),
          _EqColumn(deckId: 1, accent: .b, enabled: enabled),
        ],
      );
    }
    return Row(
      mainAxisAlignment: .center,
      crossAxisAlignment: .stretch,
      spacing: showMeters && !mono ? 4 : 8,
      children: [
        for (final deckId in const [0, 1])
          _EqColumn(
            deckId: deckId,
            accent: faderAccentForDeck(deckId)!,
            enabled: enabled,
          ),
        for (final deckId in const [0, 1])
          _VolumeColumn(
            deckId: deckId,
            accent: faderAccentForDeck(deckId)!,
            enabled: enabled,
          ),
        if (showMeters)
          _LevelMetersColumn(mono: mono, deckIds: const [0, 1, 2, 3]),
        for (final deckId in const [2, 3])
          _VolumeColumn(
            deckId: deckId,
            accent: faderAccentForDeck(deckId)!,
            enabled: enabled,
          ),
        for (final deckId in const [2, 3])
          _EqColumn(
            deckId: deckId,
            accent: faderAccentForDeck(deckId)!,
            enabled: enabled,
          ),
      ],
    );
  }
}

class _CrossfaderStrip extends ConsumerWidget {
  const new();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final enabled = ref.watch(engineRunningProvider);
    final crossfader = ref.watch(crossfaderProvider) * 100;
    return SizedBox(
      height: 24,
      child: Row(
        spacing: 10,
        children: [
          const _CrossfaderAssignmentMenu(side: CrossfaderAssign.a),
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
          const _CrossfaderAssignmentMenu(side: CrossfaderAssign.b),
        ],
      ),
    );
  }
}

class _CrossfaderAssignmentMenu extends ConsumerWidget {
  const new({required this.side});

  final CrossfaderAssign side;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final enabled = ref.watch(engineRunningProvider);
    final theme = context.theme;
    final label = side == CrossfaderAssign.a ? 'A' : 'B';
    final color = side == CrossfaderAssign.a
        ? FaderColors.a.grip
        : FaderColors.b.grip;
    return MixarMenuAnchor(
      enabled: enabled,
      minWidth: 160,
      dialogTitle: 'Crossfader $label decks',
      childBuilder: (context, controller) => MTappable(
        semanticsLabel: 'Crossfader $label deck assignments',
        onPress: enabled ? controller.toggle : null,
        builder: (context, state) => SizedBox(
          width: 32,
          height: 24,
          child: DecoratedBox(
            decoration: BoxDecoration(
              color: state.active ? theme.colors.selection : null,
              borderRadius: theme.style.borderRadius.sm,
            ),
            child: Center(
              child: Row(
                mainAxisSize: .min,
                spacing: 2,
                children: [
                  Text(
                    label,
                    style: theme.typography.body.xs.copyWith(
                      color: enabled ? color : theme.colors.mutedForeground,
                      fontWeight: .w600,
                    ),
                  ),
                  Icon(
                    LucideIcons.chevronDown,
                    size: 10,
                    color: enabled ? color : theme.colors.mutedForeground,
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
      menuBuilder: (menuContext, controller) => Consumer(
        builder: (context, menuRef, _) {
          final enabled = menuRef.watch(engineRunningProvider);
          final numDecks =
              menuRef.watch(appSettingsProvider).value?.numDecks ?? 2;
          return Column(
            mainAxisSize: .min,
            crossAxisAlignment: .stretch,
            children: [
              for (var deckId = 0; deckId < numDecks; deckId++)
                Consumer(
                  builder: (context, deckRef, _) {
                    final selected =
                        deckRef
                            .watch(deckMixerChannelProvider(deckId))
                            .crossfaderAssign ==
                        side;
                    final deckLabel = String.fromCharCode(65 + deckId);
                    return Semantics(
                      label: 'Deck $deckLabel',
                      checked: selected,
                      selected: selected,
                      child: MixarMenuItem(
                        enabled: enabled,
                        title: ExcludeSemantics(
                          child: Row(
                            children: [
                              Expanded(
                                child: Text(
                                  'Deck $deckLabel',
                                  style: TextStyle(
                                    color: FaderColors.forAccent(
                                      faderAccentForDeck(deckId)!,
                                    ).grip,
                                  ),
                                ),
                              ),
                              if (selected)
                                Icon(
                                  LucideIcons.check,
                                  size: 14,
                                  color: context.theme.colors.foreground,
                                ),
                            ],
                          ),
                        ),
                        onPress: () {
                          unawaited(
                            _mixerCmd(
                              menuContext,
                              () => setDeckCrossfaderAssign(
                                deckRef,
                                deckId,
                                selected ? CrossfaderAssign.thru : side,
                              ),
                            ),
                          );
                        },
                      ),
                    );
                  },
                ),
            ],
          );
        },
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
        RotaryKnob(
          label: 'GAIN',
          value: ch.gainTrim,
          center: kControlNormCenter,
          accentColor: accentColor,
          disabled: !enabled,
          onValueChange: (next) {
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
                  semanticLabel: '${deckDisplayLabel(deckId)} volume',
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
              color: cue ? theme.colors.primary : theme.colors.mutedForeground,
            ),
          ),
        ),
      ),
    );
  }
}

/// VU ladders between faders. Mono/stereo comes from saved settings.
class _LevelMetersColumn extends ConsumerWidget {
  const new({required this.mono, required this.deckIds});

  final bool mono;
  final List<int> deckIds;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final mode = mono ? LevelMeterMode.mono : LevelMeterMode.stereo;
    return Column(
      children: [
        const ExcludeSemantics(child: SizedBox(height: _gainHeaderHeight)),
        Expanded(
          child: Padding(
            padding: const EdgeInsets.symmetric(vertical: 4, horizontal: 2),
            child: Row(
              mainAxisAlignment: .center,
              crossAxisAlignment: .stretch,
              children: [
                for (var i = 0; i < deckIds.length; i++) ...[
                  if (i > 0) const SizedBox(width: 2),
                  RepaintBoundary(
                    child: _LiveLevelMeter(deckId: deckIds[i], mode: mode),
                  ),
                ],
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
