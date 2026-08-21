import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/mixer/deck_loop_panel.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/mixer/pad_modes.dart';
import 'package:gui_flutter/src/rust/api/engine.dart'
    show ActiveLoopInfo, EngineTransport;
import 'package:gui_flutter/src/rust/api/library.dart';

/// Watches engine/library loop state and publishes loop cmds.
class DeckLoopHost extends ConsumerStatefulWidget {
  const DeckLoopHost({
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
  ActiveLoopInfo? _syncedRegion;

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
    showFToast(context: context, variant: .destructive, title: Text('$e'));
  }

  void _syncBeatsFromRegion(ActiveLoopInfo? region, double? bpm) {
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
    required ActiveLoopInfo? activeLoop,
    required List<SavedLoopInfo> savedLoops,
    required int positionMs,
    required String? trackId,
  }) async {
    if (shiftKeyPressed() && loopActive && activeLoop != null) {
      final id = trackId;
      if (id == null || id.isEmpty) {
        return;
      }
      final slot = autoLoopSlotForBeats(_loopBeats);
      await _runLibrary(
        (library) => library.saveLoop(
          trackId: id,
          slot: slot,
          inMs: activeLoop.inMs,
          outMs: activeLoop.outMs,
        ),
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
    // Resize active region from playhead (engine `set_auto_loop`); never
    // writes the DB saved-loop row.
    await _runEngine(
      (engine) => engine.setAutoLoop(
        deckId: widget.deckId,
        beats: beats.toDouble(),
      ),
    );
  }

  Future<void> _onBeatsChip({
    required SavedLoopInfo? savedAtSlot,
    required String? trackId,
  }) async {
    if (savedAtSlot == null) {
      return;
    }
    if (shiftKeyPressed()) {
      final id = trackId;
      if (id == null || id.isEmpty) {
        return;
      }
      await _runLibrary(
        (library) => library.deleteLoop(trackId: id, slot: savedAtSlot.slot),
      );
      return;
    }
    await _runEngine(
      (engine) => engine.recallSavedLoop(
        deckId: widget.deckId,
        inMs: savedAtSlot.inMs,
        outMs: savedAtSlot.outMs,
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final trackId = ref.watch(deckTrackIdProvider(widget.deckId));
    if (trackId != _hydratedTrackId) {
      _hydratedTrackId = trackId;
      final library = _library;
      if (library != null && trackId != null && trackId.isNotEmpty) {
        unawaited(library.refreshTrack(trackId: trackId));
      }
    }

    ref.listen<ActiveLoopInfo?>(deckActiveLoopProvider(widget.deckId), (
      prev,
      next,
    ) {
      _syncBeatsFromRegion(next, ref.read(deckBpmProvider(widget.deckId)));
    });

    final activeLoop = ref.watch(deckActiveLoopProvider(widget.deckId));
    final loopActive = activeLoop != null && activeLoop.active;
    final savedLoops = ref.watch(deckSavedLoopsProvider(widget.deckId));
    final positionMs = ref.watch(deckPositionMsProvider(widget.deckId));

    final slot = autoLoopSlotForBeats(_loopBeats);
    SavedLoopInfo? savedAtSlot;
    for (final loop in savedLoops) {
      if (loop.slot == slot) {
        savedAtSlot = loop;
        break;
      }
    }

    return DeckLoopPanel(
      loopActive: loopActive,
      loopBeats: _loopBeats,
      savedLoopAtSlot: savedAtSlot != null,
      onToggleLoop: () {
        unawaited(
          _onToggleLoop(
            loopActive: loopActive,
            activeLoop: activeLoop,
            savedLoops: savedLoops,
            positionMs: positionMs,
            trackId: trackId,
          ),
        );
      },
      onHalveBeats: () {
        unawaited(
          _setBeats(
            stepAutoLoopBeats(_loopBeats, -1),
            loopActive: loopActive,
          ),
        );
      },
      onDoubleBeats: () {
        unawaited(
          _setBeats(
            stepAutoLoopBeats(_loopBeats, 1),
            loopActive: loopActive,
          ),
        );
      },
      onLoopIn: () {
        unawaited(_runEngine((e) => e.loopIn(deckId: widget.deckId)));
      },
      onLoopOut: () {
        unawaited(_runEngine((e) => e.loopOut(deckId: widget.deckId)));
      },
      onBeatsChipPress: () {
        unawaited(_onBeatsChip(savedAtSlot: savedAtSlot, trackId: trackId));
      },
      hasTrack: widget.hasTrack,
      disabled: widget.disabled,
      bordered: widget.bordered,
    );
  }
}
