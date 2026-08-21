import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/deck_cue_button.dart';
import 'package:gui_flutter/mixer/deck_performance_panel.dart';
import 'package:gui_flutter/mixer/deck_tempo_panel.dart';
import 'package:gui_flutter/mixer/deck_track_info.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/mixer/fader_slider.dart';
import 'package:gui_flutter/mixer/tempo_format.dart';
import 'package:gui_flutter/mixer/track_drop_zone.dart';
import 'package:gui_flutter/settings/settings_providers.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Deck chrome (track info, performance tabs, transport) + tempo column.
class DeckPanel extends ConsumerWidget {
  const DeckPanel({
    required this.deckId,
    required this.label,
    required this.accent,
    super.key,
  });

  final int deckId;
  final String label;
  final FaderAccent accent;

  bool get _tempoOnRight => accent == FaderAccent.a;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = context.theme;
    final accentColor = FaderColors.forAccent(accent).grip;
    final loadedTitle = ref.watch(deckTrackTitleProvider(deckId));
    final hasTrack = loadedTitle != null && loadedTitle.isNotEmpty;
    final playing = ref.watch(deckPlayingProvider(deckId));
    final skeleton = ref.watch(deckSkeletonProvider(deckId));
    final engineRunning = ref.watch(engineRunningProvider);
    final loading = ref.watch(deckLoadingProvider(deckId));
    final quantize = ref.watch(deckQuantizeProvider(deckId));
    final settings = ref.watch(appSettingsProvider).value;
    final loadDisabled = loading || !engineRunning;
    final transportDisabled = loadDisabled || !hasTrack;
    final tempo = DeckTempoPanel(
      accent: accent,
      speed: ref.watch(deckSpeedProvider(deckId)),
      tempoRange: ref.watch(deckTempoRangeProvider(deckId)),
      syncMode: ref.watch(deckSyncModeProvider(deckId)),
      isMaster: ref.watch(deckIsMasterProvider(deckId)),
      trackBpm: ref.watch(deckBpmProvider(deckId)),
      loading: skeleton,
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
      onSetMaster: () {
        unawaited(_engineCmd(context, () => setMasterDeck(ref, deckId)));
      },
    );

    final cue = DeckCueButton(
      disabled: transportDisabled,
      onBeginHold: () {
        unawaited(_engineCmd(context, () => beginDeckCueHold(ref, deckId)));
      },
      onEndHold: () {
        unawaited(_engineCmd(context, () => endDeckCueHold(ref, deckId)));
      },
      onSetCue: () {
        unawaited(_engineCmd(context, () => setDeckCuePoint(ref, deckId)));
      },
    );
    final play = Expanded(
      child: FittedBox(
        fit: .scaleDown,
        child: FButton(
          onPress: transportDisabled
              ? null
              : () {
                  unawaited(_togglePlay(context, ref, deckId));
                },
          semanticsLabel: playing ? 'Pause' : 'Play',
          child: Icon(playing ? LucideIcons.pause600 : LucideIcons.play600),
        ),
      ),
    );
    final cueWrap = Expanded(child: cue);

    final body = Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          children: [
            Text(
              label,
              style: theme.typography.body.sm.copyWith(
                fontWeight: FontWeight.w700,
                color: accentColor,
              ),
            ),
            const SizedBox(width: 4),
            DeckGainPopover(deckId: deckId, hasTrack: hasTrack),
            const Spacer(),
            _HeaderChip(
              label: 'Q',
              semanticsLabel: quantize ? 'Quantize on' : 'Quantize off',
              selected: quantize,
              disabled: transportDisabled,
              onPress: () {
                unawaited(
                  _engineCmd(
                    context,
                    () => setDeckQuantize(ref, deckId, !quantize),
                  ),
                );
              },
            ),
            const SizedBox(width: 4),
            _HeaderChip(
              label: hasTrack ? 'Eject' : 'Load',
              semanticsLabel: hasTrack ? 'Eject track' : 'Load track',
              disabled: loadDisabled,
              onPress: () {
                unawaited(
                  _engineCmd(context, () async {
                    if (hasTrack) {
                      await unloadDeck(ref, deckId);
                      return;
                    }
                    await pickTrackForDeck(ref, deckId);
                  }),
                );
              },
            ),
          ],
        ),
        const SizedBox(height: 8),
        DeckTrackInfo(deckId: deckId, hasTrack: hasTrack, title: loadedTitle),
        const SizedBox(height: 8),
        Expanded(
          child: DeckPerformancePanel(
            deckId: deckId,
            hasTrack: hasTrack,
            accent: accent,
            disabled: transportDisabled,
          ),
        ),
        const SizedBox(height: 8),
        Row(
          spacing: 8,
          children: accent == FaderAccent.a ? [cueWrap, play] : [play, cueWrap],
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
            Expanded(child: body),
            if (_tempoOnRight) ...[const SizedBox(width: 8), tempo],
          ],
        ),
      ),
    );
  }
}

class _HeaderChip extends StatelessWidget {
  const _HeaderChip({
    required this.label,
    required this.onPress,
    this.semanticsLabel,
    this.selected = false,
    this.disabled = false,
  });

  final String label;
  final VoidCallback onPress;
  final String? semanticsLabel;
  final bool selected;
  final bool disabled;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final fg = disabled
        ? theme.colors.mutedForeground
        : theme.colors.foreground;
    return Semantics(
      button: true,
      enabled: !disabled,
      label: semanticsLabel ?? label,
      child: GestureDetector(
        onTap: disabled ? null : onPress,
        child: DecoratedBox(
          decoration: BoxDecoration(
            color: selected ? theme.colors.secondary : theme.colors.background,
            border: Border.all(color: theme.colors.border),
            borderRadius: theme.style.borderRadius.sm,
          ),
          child: Padding(
            padding: const .symmetric(horizontal: 8, vertical: 4),
            child: Text(
              label,
              style: theme.typography.body.xs.copyWith(
                fontWeight: .w600,
                color: fg,
              ),
            ),
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
    showFToast(context: context, variant: .destructive, title: Text('$e'));
  }
}

Future<void> _togglePlay(
  BuildContext context,
  WidgetRef ref,
  int deckId,
) async {
  await _engineCmd(context, () => toggleDeckPlay(ref, deckId));
}
