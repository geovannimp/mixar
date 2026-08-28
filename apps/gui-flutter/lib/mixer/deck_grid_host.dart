import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/mixer/deck_grid_panel.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/mixer/waveform/beat_grid.dart';
import 'package:gui_flutter/mixer/waveform/waveform_providers.dart';
import 'package:gui_flutter/src/rust/api/library.dart';

/// Watches deck/library beat grid state and publishes grid edits.
class DeckGridHost extends ConsumerWidget {
  const DeckGridHost({
    required this.deckId,
    this.hasTrack = false,
    this.disabled = false,
    this.bordered = true,
    super.key,
  });

  final int deckId;
  final bool hasTrack;
  final bool disabled;
  final bool bordered;

  Future<bool> _saveGrid(
    BuildContext context,
    WidgetRef ref, {
    required String trackId,
    required double bpm,
    required double firstBeatSecs,
  }) async {
    final library = ref.read(libraryTransportProvider).asData?.value;
    if (library == null) {
      return false;
    }
    try {
      await library.saveBeatGrid(
        trackId: trackId,
        bpm: bpm,
        firstBeatSecs: firstBeatSecs,
      );
      return true;
    } catch (e) {
      if (context.mounted) {
        showFToast(context: context, variant: .destructive, title: Text('$e'));
      }
      return false;
    }
  }

  Future<void> _edit(
    BuildContext context,
    WidgetRef ref, {
    required String trackId,
    required BeatGridData? grid,
    required double? bpmOverride,
    required double? firstBeatOverride,
  }) async {
    final bpm = bpmOverride ?? grid?.bpm ?? defaultGridBpm;
    final firstBeatSecs = firstBeatOverride ?? beatGridFirstBeatSecs(grid);
    await _saveGrid(
      context,
      ref,
      trackId: trackId,
      bpm: bpm,
      firstBeatSecs: firstBeatSecs,
    );
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final trackId = ref.watch(deckTrackIdProvider(deckId));
    final grid = trackId == null
        ? null
        : ref.watch(beatGridProvider(trackId));
    final positionMs = ref.watch(deckPositionMsProvider(deckId));
    final panelDisabled = disabled || trackId == null;

    return DeckGridPanel(
      bpm: grid?.bpm,
      hasTrack: hasTrack && trackId != null,
      disabled: panelDisabled,
      bordered: bordered,
      onSetDownbeat: () {
        if (trackId == null) {
          return;
        }
        unawaited(
          _edit(
            context,
            ref,
            trackId: trackId,
            grid: grid,
            bpmOverride: null,
            firstBeatOverride: positionMs / 1000.0,
          ),
        );
      },
      onNudgeBack: () {
        if (trackId == null) {
          return;
        }
        unawaited(
          _edit(
            context,
            ref,
            trackId: trackId,
            grid: grid,
            bpmOverride: null,
            firstBeatOverride: nudgeFirstBeatSecs(
              beatGridFirstBeatSecs(grid),
              -kGridNudgeMs,
            ),
          ),
        );
      },
      onNudgeForward: () {
        if (trackId == null) {
          return;
        }
        unawaited(
          _edit(
            context,
            ref,
            trackId: trackId,
            grid: grid,
            bpmOverride: null,
            firstBeatOverride: nudgeFirstBeatSecs(
              beatGridFirstBeatSecs(grid),
              kGridNudgeMs,
            ),
          ),
        );
      },
      onBpmDown: () {
        if (trackId == null) {
          return;
        }
        final bpm = stepGridBpm(grid?.bpm ?? defaultGridBpm, -kGridBpmStep);
        unawaited(
          _edit(
            context,
            ref,
            trackId: trackId,
            grid: grid,
            bpmOverride: bpm,
            firstBeatOverride: null,
          ),
        );
      },
      onBpmUp: () {
        if (trackId == null) {
          return;
        }
        final bpm = stepGridBpm(grid?.bpm ?? defaultGridBpm, kGridBpmStep);
        unawaited(
          _edit(
            context,
            ref,
            trackId: trackId,
            grid: grid,
            bpmOverride: bpm,
            firstBeatOverride: null,
          ),
        );
      },
      onBpmSubmit: (bpm) {
        if (trackId == null) {
          return;
        }
        unawaited(
          _edit(
            context,
            ref,
            trackId: trackId,
            grid: grid,
            bpmOverride: bpm,
            firstBeatOverride: null,
          ),
        );
      },
    );
  }
}
