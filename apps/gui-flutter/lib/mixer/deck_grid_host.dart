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
class DeckGridHost extends ConsumerStatefulWidget {
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

  @override
  ConsumerState<DeckGridHost> createState() => _DeckGridHostState();
}

class _DeckGridHostState extends ConsumerState<DeckGridHost> {
  String? _pendingTrackId;
  double? _pendingBpm;
  double? _pendingFirstBeat;
  Future<void> _saveChain = Future<void>.value();

  void _clearPending() {
    if (_pendingTrackId == null &&
        _pendingBpm == null &&
        _pendingFirstBeat == null) {
      return;
    }
    setState(() {
      _pendingTrackId = null;
      _pendingBpm = null;
      _pendingFirstBeat = null;
    });
  }

  void _toast(Object message) {
    if (!mounted) {
      return;
    }
    showFToast(
      context: context,
      variant: .destructive,
      title: Text('$message'),
    );
  }

  double _baseBpm(BeatGridData? grid) =>
      _pendingBpm ?? grid?.bpm ?? defaultGridBpm;

  double _baseFirstBeat(BeatGridData? grid) =>
      _pendingFirstBeat ?? beatGridFirstBeatSecs(grid);

  Future<void> _enqueueEdit({
    required String trackId,
    required BeatGridData? grid,
    required double? bpmOverride,
    required double? firstBeatOverride,
  }) {
    final bpm = bpmOverride ?? _baseBpm(grid);
    final firstBeatSecs = firstBeatOverride ?? _baseFirstBeat(grid);
    setState(() {
      _pendingTrackId = trackId;
      _pendingBpm = bpm;
      _pendingFirstBeat = firstBeatSecs;
    });

    final save = _saveChain.then((_) async {
      if (!mounted || _pendingTrackId != trackId) {
        return;
      }
      final library = ref.read(libraryTransportProvider).asData?.value;
      if (library == null) {
        _toast('Library not ready');
        return;
      }
      try {
        await library.saveBeatGrid(
          trackId: trackId,
          bpm: bpm,
          firstBeatSecs: firstBeatSecs,
        );
      } catch (e) {
        _toast(e);
      }
    });
    _saveChain = save.catchError((_) {});
    return save;
  }

  @override
  Widget build(BuildContext context) {
    final trackId = ref.watch(deckTrackIdProvider(widget.deckId));
    final libraryReady = ref.watch(libraryTransportProvider).hasValue;
    final grid = trackId == null ? null : ref.watch(beatGridProvider(trackId));
    final positionMs = ref.watch(deckPositionMsProvider(widget.deckId));

    ref.listen(deckTrackIdProvider(widget.deckId), (prev, next) {
      if (prev != next) {
        _clearPending();
      }
    });
    if (trackId != null) {
      ref.listen(beatGridProvider(trackId), (prev, next) {
        if (_pendingTrackId != trackId ||
            _pendingBpm == null ||
            next?.bpm == null) {
          return;
        }
        final first = beatGridFirstBeatSecs(next);
        if ((next!.bpm! - _pendingBpm!).abs() < 1e-6 &&
            (first - (_pendingFirstBeat ?? 0)).abs() < 1e-4) {
          _clearPending();
        }
      });
    }

    final displayBpm = _pendingTrackId == trackId
        ? (_pendingBpm ?? grid?.bpm)
        : grid?.bpm;
    final panelDisabled = widget.disabled || trackId == null || !libraryReady;

    return DeckGridPanel(
      bpm: displayBpm,
      hasTrack: widget.hasTrack && trackId != null,
      disabled: panelDisabled,
      bordered: widget.bordered,
      onSetDownbeat: () {
        if (trackId == null) {
          return;
        }
        unawaited(
          _enqueueEdit(
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
          _enqueueEdit(
            trackId: trackId,
            grid: grid,
            bpmOverride: null,
            firstBeatOverride: nudgeFirstBeatSecs(
              _baseFirstBeat(grid),
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
          _enqueueEdit(
            trackId: trackId,
            grid: grid,
            bpmOverride: null,
            firstBeatOverride: nudgeFirstBeatSecs(
              _baseFirstBeat(grid),
              kGridNudgeMs,
            ),
          ),
        );
      },
      onBpmDown: () {
        if (trackId == null) {
          return;
        }
        unawaited(
          _enqueueEdit(
            trackId: trackId,
            grid: grid,
            bpmOverride: stepGridBpm(_baseBpm(grid), -kGridBpmStep),
            firstBeatOverride: null,
          ),
        );
      },
      onBpmUp: () {
        if (trackId == null) {
          return;
        }
        unawaited(
          _enqueueEdit(
            trackId: trackId,
            grid: grid,
            bpmOverride: stepGridBpm(_baseBpm(grid), kGridBpmStep),
            firstBeatOverride: null,
          ),
        );
      },
      onBpmSubmit: (bpm) {
        if (trackId == null) {
          return;
        }
        unawaited(
          _enqueueEdit(
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
