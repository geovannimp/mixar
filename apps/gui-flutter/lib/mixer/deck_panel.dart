import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/deck_performance_panel.dart';
import 'package:gui_flutter/mixer/deck_tempo_panel.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/mixer/fader_slider.dart';
import 'package:gui_flutter/mixer/tempo_format.dart';
import 'package:gui_flutter/mixer/track_drop_zone.dart';
import 'package:gui_flutter/mixer/waveform/overview_strip.dart';
import 'package:gui_flutter/settings/settings_providers.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:skeletonizer/skeletonizer.dart';

/// Placeholder deck chrome (track info, performance tabs, transport) + tempo column.
class DeckPanel extends ConsumerWidget {
  const DeckPanel({
    required this.deckId,
    required this.label,
    required this.accent,
    required this.isMaster,
    required this.onMasterChanged,
    super.key,
  });

  final int deckId;
  final String label;
  final FaderAccent accent;
  final bool isMaster;
  final ValueChanged<bool> onMasterChanged;

  bool get _tempoOnRight => accent == FaderAccent.a;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = context.theme;
    final accentColor = FaderColors.forAccent(accent).grip;
    final loadedTitle = ref.watch(deckTrackTitleProvider(deckId));
    final hasTrack = loadedTitle != null && loadedTitle.isNotEmpty;
    final playing = ref.watch(deckPlayingProvider(deckId));
    final skeleton = ref.watch(deckSkeletonProvider(deckId));
    final settings = ref.watch(appSettingsProvider).value;
    final tempo = DeckTempoPanel(
      accent: accent,
      isMaster: isMaster,
      onMasterChanged: onMasterChanged,
      trackBpm: ref.watch(deckBpmProvider(deckId)),
      loading: skeleton,
      defaultTempoRange: settings?.defaultTempoRange ?? kDefaultTempoRange,
      tempoRangeSteps: settings?.tempoRangeSteps ?? kTempoRangeSteps,
    );

    final body = Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text(
          label,
          style: theme.typography.body.sm.copyWith(
            fontWeight: FontWeight.w700,
            color: accentColor,
          ),
        ),
        Skeletonizer(
          enabled: skeleton,
          child: Text(
            loadedTitle ?? 'No track loaded',
            style: theme.typography.body.xs.copyWith(
              color: theme.colors.mutedForeground,
            ),
          ),
        ),
        const SizedBox(height: 8),
        OverviewStrip(deckId: deckId, height: 36),
        const SizedBox(height: 8),
        Expanded(
          child: DeckPerformancePanel(deckId: deckId, hasTrack: hasTrack),
        ),
        const SizedBox(height: 8),
        Row(
          children: [
            Expanded(
              child: FButton(
                variant: .secondary,
                onPress: () {},
                child: const Text('Cue'),
              ),
            ),
            const SizedBox(width: 8),
            Expanded(
              child: FButton(
                onPress: !hasTrack
                    ? null
                    : () {
                        unawaited(_togglePlay(context, ref, deckId));
                      },
                semanticsLabel: playing ? 'Pause' : 'Play',
                child: Icon(
                  playing ? LucideIcons.pause600 : LucideIcons.play600,
                ),
              ),
            ),
          ],
        ),
      ],
    );

    return TrackDropZone(
      deckId: deckId,
      child: Padding(
        padding: const EdgeInsets.all(12),
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

Future<void> _togglePlay(
  BuildContext context,
  WidgetRef ref,
  int deckId,
) async {
  try {
    await toggleDeckPlay(ref, deckId);
  } catch (e) {
    if (!context.mounted) {
      return;
    }
    showFToast(context: context, variant: .destructive, title: Text('$e'));
  }
}
