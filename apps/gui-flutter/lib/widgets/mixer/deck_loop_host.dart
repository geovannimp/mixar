import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/models/bus_enums.dart';
import 'package:gui_flutter/core/models/pad_modes.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/src/rust/api/engine.dart';
import 'package:gui_flutter/src/rust/api/event_mirrors.dart' show SavedLoop;
import 'package:gui_flutter/src/rust/api/library.dart' show LibraryTransport;
import 'package:gui_flutter/widgets/mixer/deck_loop_panel.dart';
import 'package:gui_flutter/widgets/shell/mixar_toast.dart';

/// Watches engine/library loop state and publishes loop cmds.
class DeckLoopHost extends ConsumerStatefulWidget {
  const new({
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
  ConsumerState<DeckLoopHost> createState() => _DeckLoopHostState();
}

class _DeckLoopHostState extends ConsumerState<DeckLoopHost> {
  var _loopBeats = 4;
  String? _hydratedTrackId;
  LoopRegion? _syncedRegion;

  EngineTransport? get _engine =>
      ref.read(engineTransportProvider).asData?.value;

  LibraryTransport? get _library =>
      ref.read(libraryTransportProvider).asData?.value;

  Future<bool> _runEngine(
    Future<void> Function(EngineTransport engine) fn,
  ) async {
    final engine = _engine;
    if (engine == null) {
      return false;
    }
    try {
      await fn(engine);
      return true;
    } catch (e) {
      _toastError(e);
      return false;
    }
  }

  Future<bool> _runLibrary(
    Future<void> Function(LibraryTransport library) fn,
  ) async {
    final library = _library;
    if (library == null) {
      return false;
    }
    try {
      await fn(library);
      return true;
    } catch (e) {
      _toastError(e);
      return false;
    }
  }

  void _toastError(Object e) {
    if (!mounted) {
      return;
    }
    showMixarToast(
      context: context,
      variant: MixarToastVariant.destructive,
      title: Text('$e'),
    );
  }

  void _syncBeatsFromRegion(LoopRegion? region, double? bpm) {
    if (region == null) {
      _syncedRegion = null;
      return;
    }
    if (_syncedRegion?.inMs == region.inMs &&
        _syncedRegion?.outMs == region.outMs) {
      return;
    }
    _syncedRegion = region;
    final next = beatsFromLoopMs(
      inMs: region.inMs,
      outMs: region.outMs,
      bpm: bpm,
    );
    if (next != _loopBeats) {
      setState(() => _loopBeats = next);
    }
  }

  Future<void> _onToggleLoop({
    required bool loopActive,
    required List<SavedLoop> savedLoops,
    required int positionMs,
    required String? trackId,
  }) async {
    if (shiftKeyPressed()) {
      final id = trackId;
      if (id == null || id.isEmpty) {
        return;
      }
      final loop = savedLoopAtPosition(savedLoops, positionMs);
      if (loop == null) {
        return;
      }
      if (loopActive) {
        await _runEngine((engine) => engine.exitLoop(deckId: widget.deckId));
      }
      await _runLibrary(
        (library) => library.deleteLoop(trackId: id, slot: loop.slot),
      );
      return;
    }

    if (loopActive) {
      await _runEngine((engine) => engine.exitLoop(deckId: widget.deckId));
      return;
    }

    final underPlayhead = savedLoopAtPosition(savedLoops, positionMs);
    if (underPlayhead != null) {
      await _runEngine(
        (engine) => engine.recallSavedLoop(
          deckId: widget.deckId,
          inMs: underPlayhead.inMs,
          outMs: underPlayhead.outMs,
        ),
      );
      return;
    }

    await _runEngine(
      (engine) => engine.setAutoLoop(
        deckId: widget.deckId,
        beats: _loopBeats.toDouble(),
      ),
    );
  }

  Future<void> _setBeats(int beats, {required bool loopActive}) async {
    setState(() => _loopBeats = beats);
    if (!loopActive) {
      return;
    }
    // Resize the active region; the engine auto-saves the updated loop.
    await _runEngine(
      (engine) =>
          engine.setAutoLoop(deckId: widget.deckId, beats: beats.toDouble()),
    );
  }

  void _hydrateTrack(String? trackId) {
    if (trackId == _hydratedTrackId) {
      return;
    }
    final library = _library;
    if (library == null) {
      // Library not ready yet; leave `_hydratedTrackId` unset so the
      // library-ready listener retries with this track.
      return;
    }
    _hydratedTrackId = trackId;
    if (trackId != null && trackId.isNotEmpty) {
      unawaited(
        _runLibrary((library) => library.refreshTrack(trackId: trackId)),
      );
    }
  }

  @override
  void initState() {
    super.initState();
    ref.listenManual<String?>(
      deckTrackIdProvider(widget.deckId),
      (prev, next) => _hydrateTrack(next),
      fireImmediately: true,
    );
    ref.listenManual<AsyncValue<LibraryTransport>>(libraryTransportProvider, (
      prev,
      next,
    ) {
      if (next.asData?.value != null) {
        _hydrateTrack(ref.read(deckTrackIdProvider(widget.deckId)));
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    ref
      ..watch(engineTransportProvider)
      ..watch(libraryTransportProvider);
    final trackId = ref.watch(deckTrackIdProvider(widget.deckId));

    ref.listen<LoopRegion?>(deckActiveLoopProvider(widget.deckId), (
      prev,
      next,
    ) {
      _syncBeatsFromRegion(next, ref.read(deckBpmProvider(widget.deckId)));
    });

    final activeLoop = ref.watch(deckActiveLoopProvider(widget.deckId));
    final loopActive = activeLoop != null && activeLoop.active;
    final loopPending =
        ref.watch(deckPendingLoopInMsProvider(widget.deckId)) != null;
    final savedLoops = ref.watch(deckSavedLoopsProvider(widget.deckId));
    final positionMs = ref.watch(deckPositionMsProvider(widget.deckId));

    return DeckLoopPanel(
      loopActive: loopActive,
      loopPending: loopPending,
      loopBeats: _loopBeats,
      onToggleLoop: () {
        unawaited(
          _onToggleLoop(
            loopActive: loopActive,
            savedLoops: savedLoops,
            positionMs: positionMs,
            trackId: trackId,
          ),
        );
      },
      onHalveBeats: () {
        unawaited(
          _setBeats(stepAutoLoopBeats(_loopBeats, -1), loopActive: loopActive),
        );
      },
      onDoubleBeats: () {
        unawaited(
          _setBeats(stepAutoLoopBeats(_loopBeats, 1), loopActive: loopActive),
        );
      },
      onLoopIn: () {
        final ms = ref.read(deckPositionMsProvider(widget.deckId));
        unawaited(
          _runEngine((e) => e.loopIn(deckId: widget.deckId, positionMs: ms)),
        );
      },
      onLoopOut: () {
        final ms = ref.read(deckPositionMsProvider(widget.deckId));
        unawaited(
          _runEngine((e) => e.loopOut(deckId: widget.deckId, positionMs: ms)),
        );
      },
      hasTrack: widget.hasTrack,
      disabled: widget.disabled,
      bordered: widget.bordered,
    );
  }
}
