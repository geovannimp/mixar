import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/theme/app_typography.dart';
import 'package:gui_flutter/core/utils/deck_cue_hold_session.dart';
import 'package:gui_flutter/core/utils/mixer_live_color.dart';
import 'package:gui_flutter/core/utils/tempo_format.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/src/rust/api/engine.dart' show EngineTransport;
import 'package:gui_flutter/widgets/mixer/deck_cue_button.dart';
import 'package:gui_flutter/widgets/mixer/deck_gain_popover.dart';
import 'package:gui_flutter/widgets/mixer/deck_performance_panel.dart';
import 'package:gui_flutter/widgets/mixer/deck_tempo_panel.dart';
import 'package:gui_flutter/widgets/mixer/deck_track_info.dart';
import 'package:gui_flutter/widgets/mixer/fader_slider.dart';
import 'package:gui_flutter/widgets/mixer/mixer_button.dart';
import 'package:gui_flutter/widgets/mixer/track_drop_zone.dart';
import 'package:gui_flutter/widgets/shell/app_tooltip.dart';
import 'package:gui_flutter/widgets/shell/m_tappable.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:gui_flutter/widgets/shell/mixar_toast.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Deck chrome (track info, performance tabs, transport) + tempo column.
class DeckPanel extends ConsumerWidget {
  const new({
    required this.deckId,
    required this.label,
    required this.accent,
    super.key,
  });

  final int deckId;
  final String label;
  final FaderAccent accent;

  bool get _tempoOnRight => deckId.isEven;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = context.theme;
    final accentColor = FaderColors.forAccent(accent).grip;
    final loadedTitle = ref.watch(deckTrackTitleProvider(deckId));
    final hasTrack = ref.watch(deckHasTrackProvider(deckId));
    final skeleton = ref.watch(deckSkeletonProvider(deckId));
    final engineRunning = ref.watch(engineRunningProvider);
    final loading = ref.watch(deckLoadingProvider(deckId));
    final quantize = ref.watch(deckQuantizeProvider(deckId));
    final fxRouted = ref.watch(deckFxRoutedProvider(deckId));
    final slip = ref.watch(deckSlipEnabledProvider(deckId));
    final settings = ref.watch(appSettingsProvider).value;
    final loadDisabled = loading || !engineRunning;
    final transportDisabled = loadDisabled || !hasTrack;
    final isMaster = ref.watch(deckIsMasterProvider(deckId));
    final tempo = DeckTempoPanel(
      accent: accent,
      speed: ref.watch(deckSpeedProvider(deckId)),
      tempoRange: ref.watch(deckTempoRangeProvider(deckId)),
      syncMode: ref.watch(deckSyncModeProvider(deckId)),
      isMaster: isMaster,
      trackBpm: ref.watch(deckBpmProvider(deckId)),
      loading: skeleton,
      enabled: engineRunning,
      tempoRangeSteps: settings?.tempoRangeSteps ?? kTempoRangeSteps,
      onSpeedChange: (speed) {
        unawaited(_engineCmd(context, () => setDeckSpeed(ref, deckId, speed)));
      },
      onTempoRangeChange: (tempoRange) {
        unawaited(
          _engineCmd(context, () => setDeckTempoRange(ref, deckId, tempoRange)),
        );
      },
      onToggleSync: (beatSync) {
        unawaited(
          _engineCmd(
            context,
            () => toggleDeckSync(ref, deckId, beatSync: beatSync),
          ),
        );
      },
    );

    // Play/pause must not rebuild DeckTrackInfo / pads (artwork blink).
    final play = Expanded(
      child: _DeckPlayButton(deckId: deckId, disabled: transportDisabled),
    );
    final cueWrap = Expanded(
      child: _DeckCueControl(deckId: deckId, disabled: transportDisabled),
    );

    final body = Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      spacing: 8,
      children: [
        LayoutBuilder(
          builder: (context, constraints) => SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: ConstrainedBox(
              constraints: BoxConstraints(minWidth: constraints.maxWidth),
              child: IntrinsicWidth(
                child: Row(
                  children: [
                    Text(
                      label.toUpperCase(),
                      style: theme.typography.display.xs.copyWith(
                        fontFamily: MixarFonts.spaceGrotesk,
                        fontWeight: FontWeight.w700,
                        letterSpacing: 0.6,
                        height: 1,
                        color: accentColor,
                      ),
                    ),
                    const SizedBox(width: 4),
                    DeckGainPopover(deckId: deckId, hasTrack: hasTrack),
                    const Spacer(),
                    _MasterButton(
                      isMaster: isMaster,
                      disabled: loadDisabled,
                      onPress: () {
                        unawaited(
                          _engineCmd(context, () => setMasterDeck(ref, deckId)),
                        );
                      },
                    ),
                    const SizedBox(width: 4),
                    _QuantizeButton(
                      quantize: quantize,
                      disabled: loadDisabled,
                      onPress: () {
                        // Capture the desired state before the async command.
                        final enabled = !quantize;
                        unawaited(
                          _engineCmd(
                            context,
                            () => setDeckQuantize(ref, deckId, enabled),
                          ),
                        );
                      },
                    ),
                    const SizedBox(width: 4),
                    _SlipButton(
                      slip: slip,
                      disabled: loadDisabled,
                      onPress: () {
                        final enabled = !slip;
                        unawaited(
                          _engineCmd(
                            context,
                            () => setDeckSlip(ref, deckId, enabled),
                          ),
                        );
                      },
                    ),
                    const SizedBox(width: 4),
                    _FxButton(
                      routed: fxRouted,
                      disabled: !engineRunning,
                      onPress: () {
                        // Capture the desired state before the async command.
                        final routed = !fxRouted;
                        unawaited(
                          _engineCmd(
                            context,
                            () => setDeckFxRouted(ref, deckId, routed),
                          ),
                        );
                      },
                    ),
                  ],
                ),
              ),
            ),
          ),
        ),
        DeckTrackInfo(deckId: deckId, hasTrack: hasTrack, title: loadedTitle),
        Expanded(
          child: DeckPerformancePanel(
            deckId: deckId,
            hasTrack: hasTrack,
            accent: accent,
            // Engine-level only: pad mode must stay selectable while the deck
            // is empty, and the per-surface panels already gate their controls
            // on `hasTrack`.
            disabled: loadDisabled,
          ),
        ),
        Row(
          spacing: 8,
          children: _tempoOnRight ? [cueWrap, play] : [play, cueWrap],
        ),
      ],
    );

    return TrackDropZone(
      deckId: deckId,
      child: ClipRect(
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            if (!_tempoOnRight) ...[tempo, const SizedBox(width: 8)],
            Expanded(
              // The tempo rail spans the full deck height; only the deck body
              // keeps the inset.
              child: Padding(
                padding: const EdgeInsets.symmetric(vertical: 10),
                child: body,
              ),
            ),
            if (_tempoOnRight) ...[const SizedBox(width: 8), tempo],
          ],
        ),
      ),
    );
  }
}

/// Owns one ordered cue-hold session so begin/end never race.
class _DeckCueControl extends ConsumerStatefulWidget {
  const new({required this.deckId, required this.disabled});

  final int deckId;
  final bool disabled;

  @override
  ConsumerState<_DeckCueControl> createState() => _DeckCueControlState();
}

class _DeckCueControlState extends ConsumerState<_DeckCueControl> {
  final _session = DeckCueHoldSession();
  // Cached via listenManual — Riverpod forbids ref.read in dispose().
  EngineTransport? _engine;
  ProviderSubscription<AsyncValue<EngineTransport?>>? _engineListen;

  @override
  void initState() {
    super.initState();
    _engineListen = ref.listenManual(engineTransportProvider, (prev, next) {
      _engine = next.asData?.value;
    }, fireImmediately: true);
  }

  void _beginHold() {
    final deckId = widget.deckId;
    _session.begin(
      () => _engineCmd(context, () => beginDeckCueHold(ref, deckId)),
    );
  }

  void _endHold() {
    final deckId = widget.deckId;
    _session.end(() => _engineCmd(context, () => endDeckCueHold(ref, deckId)));
  }

  void _setCue() {
    unawaited(_engineCmd(context, () => setDeckCuePoint(ref, widget.deckId)));
  }

  @override
  void dispose() {
    final engine = _engine;
    final deckId = widget.deckId;
    // Fire-and-forget: dispose cannot await. Engine transport is kept alive
    // for the app session, so this endCueHold should still land.
    unawaited(
      _session
          .disposeEnd(() async {
            await engine?.endCueHold(deckId: deckId);
          })
          .catchError((Object e) {
            debugPrint('DeckCueControl: endCueHold failed during dispose: $e');
          }),
    );
    _engineListen?.close();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return DeckCueButton(
      disabled: widget.disabled,
      onBeginHold: _beginHold,
      onEndHold: _endHold,
      onSetCue: _setCue,
    );
  }
}

class _DeckPlayButton extends ConsumerWidget {
  const new({required this.deckId, required this.disabled});

  final int deckId;
  final bool disabled;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = context.theme;
    final playing = ref.watch(deckPlayingProvider(deckId));
    final playLabel = playing ? 'Pause' : 'Play';
    // Outline transport chrome (website stem play parity): hairline + fg at
    // rest; accent border/icon on hover, press, and while playing.
    return AppTooltip(
      tip: playLabel,
      child: MTappable(
        selected: playing,
        onPress: disabled
            ? null
            : () {
                final wantPlaying = !playing;
                unawaited(_setPlaying(context, ref, deckId, wantPlaying));
              },
        semanticsLabel: playLabel,
        builder: (context, state) {
          final accented = state.active;
          final borderColor = accented
              ? theme.colors.accent
              : theme.colors.border;
          final fg = accented ? theme.colors.accent : theme.colors.foreground;
          // Soft solid teal only while playing — hover stays outline-only.
          final fill = playing ? theme.colors.accentTint : theme.colors.card;
          return Opacity(
            opacity: state.disabled ? 0.5 : 1,
            child: DecoratedBox(
              decoration: BoxDecoration(
                color: fill,
                border: Border.all(
                  color: borderColor,
                  width: theme.style.borderWidth,
                ),
                borderRadius: theme.style.borderRadius.md,
              ),
              child: ConstrainedBox(
                constraints: const BoxConstraints(minHeight: 36),
                child: Center(
                  // Optical nudge for the play triangle (site translate-x-px).
                  child: Transform.translate(
                    offset: playing ? Offset.zero : const Offset(1, 0),
                    child: Icon(
                      playing ? LucideIcons.pause : LucideIcons.play,
                      size: 18,
                      color: fg,
                    ),
                  ),
                ),
              ),
            ),
          );
        },
      ),
    );
  }
}

class _MasterButton extends StatelessWidget {
  const new({
    required this.onPress,
    required this.isMaster,
    this.disabled = false,
  });

  final VoidCallback onPress;
  final bool isMaster;
  final bool disabled;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final on = isMaster && !disabled;
    final tip = isMaster ? 'Master' : 'Set master';
    return AppTooltip(
      tip: tip,
      description: isMaster
          ? 'This deck is the sync reference.'
          : 'Make this deck the tempo reference for sync.',
      child: MixerButton(
        variant: .outline,
        size: .xs,
        mainAxisSize: .min,
        selected: on,
        onPress: (!disabled && !isMaster) ? onPress : null,
        semanticsLabel: tip,
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 2),
        backgroundColor: on ? kMixerMasterFill : null,
        child: Icon(
          LucideIcons.crown,
          size: 14,
          color: on ? kMixerMasterOn : theme.colors.mutedForeground,
        ),
      ),
    );
  }
}

class _QuantizeButton extends StatelessWidget {
  const new({
    required this.onPress,
    required this.quantize,
    this.disabled = false,
  });

  final VoidCallback onPress;
  final bool quantize;
  final bool disabled;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final on = quantize && !disabled;
    final tip = quantize ? 'Quantize on' : 'Quantize off';
    return AppTooltip(
      tip: tip,
      description: 'Snap cues, loops, and hot cues to the beat grid.',
      child: MixerButton(
        variant: .outline,
        size: .xs,
        mainAxisSize: .min,
        selected: on,
        onPress: disabled ? null : onPress,
        semanticsLabel: tip,
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 2),
        backgroundColor: on ? theme.colors.primaryTint : null,
        child: Text(
          'Q',
          style: TextStyle(
            fontWeight: .w600,
            color: on ? kMixerLiveOn : theme.colors.mutedForeground,
          ),
        ),
      ),
    );
  }
}

class _SlipButton extends StatelessWidget {
  const new({required this.onPress, required this.slip, this.disabled = false});

  final VoidCallback onPress;
  final bool slip;
  final bool disabled;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    const slipOn = Color.fromARGB(191, 96, 165, 250);
    const slipFill = Color.fromARGB(28, 96, 165, 250);
    final on = slip && !disabled;
    final tip = slip ? 'Slip on' : 'Slip off';
    return AppTooltip(
      tip: tip,
      description: 'Shadow playhead keeps moving during loops and scratch; catch up on exit.',
      child: MixerButton(
        variant: .outline,
        size: .xs,
        mainAxisSize: .min,
        selected: on,
        onPress: disabled ? null : onPress,
        semanticsLabel: tip,
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 2),
        backgroundColor: on ? slipFill : null,
        child: Text(
          'S',
          style: TextStyle(
            fontWeight: .w600,
            color: on ? slipOn : theme.colors.mutedForeground,
          ),
        ),
      ),
    );
  }
}

class _FxButton extends StatelessWidget {
  const new({
    required this.onPress,
    required this.routed,
    this.disabled = false,
  });

  final VoidCallback onPress;
  final bool routed;
  final bool disabled;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final on = routed && !disabled;
    final tip = routed ? 'FX on' : 'FX off';
    return AppTooltip(
      tip: tip,
      description: 'Route this deck through the FX unit.',
      child: MixerButton(
        variant: .outline,
        size: .xs,
        mainAxisSize: .min,
        selected: on,
        onPress: disabled ? null : onPress,
        semanticsLabel: tip,
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 2),
        backgroundColor: on ? theme.colors.primaryTint : null,
        child: Text(
          'FX',
          style: TextStyle(
            fontWeight: .w600,
            color: on ? kMixerLiveOn : theme.colors.mutedForeground,
          ),
        ),
      ),
    );
  }
}

Future<void> _engineCmd(
  BuildContext context,
  Future<void> Function() fn,
) async {
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

Future<void> _setPlaying(
  BuildContext context,
  WidgetRef ref,
  int deckId,
  bool playing,
) async {
  await _engineCmd(context, () => setDeckPlaying(ref, deckId, playing));
}
