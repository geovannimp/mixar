import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/mixer/deck_pads_panel.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/mixer/pad_modes.dart';
import 'package:gui_flutter/mixer/pads/sampler_pads.dart';
import 'package:gui_flutter/mixer/track_drag.dart';
import 'package:gui_flutter/settings/settings_providers.dart';
import 'package:gui_flutter/shell/mixar_toast.dart';
import 'package:gui_flutter/src/rust/api/engine.dart' as rust;
import 'package:gui_flutter/src/rust/api/library.dart';

/// Watches engine/library providers and publishes named pad press/release cmds.
class DeckPadsHost extends ConsumerStatefulWidget {
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
  ConsumerState<DeckPadsHost> createState() => _DeckPadsHostState();
}

class _DeckPadsHostState extends ConsumerState<DeckPadsHost> {
  String? _hydratedTrackId;

  rust.EngineTransport? get _engine =>
      ref.read(engineTransportProvider).asData?.value;

  rust.PadMode _toEnginePadMode(PadMode mode) => switch (mode) {
    PadMode.hotCue => rust.PadMode.hotCue,
    PadMode.loopRoll => rust.PadMode.loopRoll,
    PadMode.beatJump => rust.PadMode.beatJump,
    PadMode.sampler => rust.PadMode.sampler,
    PadMode.stems => rust.PadMode.stems,
    PadMode.keyboard => rust.PadMode.keyboard,
    PadMode.keyShift => rust.PadMode.keyShift,
  };

  Future<bool> _run(
    Future<void> Function(rust.EngineTransport engine) fn,
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
    }
    return false;
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

  /// Step the page bar of whichever pad mode owns it, wrapping at both ends.
  ///
  /// Keyboard and Key Shift keep independent pages, so only the active mode's
  /// value moves; [direction] is `-1` for previous and `1` for next.
  void _stepPage(
    int direction,
    PadMode mode,
    int keyboardPage,
    int keyShiftPage,
  ) {
    final keyShift = mode == PadMode.keyShift;
    final page = keyShift ? keyShiftPage : keyboardPage;
    final count = keyShift ? kKeyShiftPageCount : kKeyboardPageCount;
    final next = direction < 0
        ? (page <= 1 ? count : page - 1)
        : (page >= count ? 1 : page + 1);
    unawaited(
      _run(
        (engine) => keyShift
            ? engine.setKeyShiftPage(deckId: widget.deckId, page: next)
            : engine.setKeyboardPage(deckId: widget.deckId, page: next),
      ),
    );
  }

  List<SamplerSlot> _slotsFromChrome(List<rust.SamplerSlotChrome> chrome) {
    return [
      for (var i = 0; i < 8; i++)
        if (i < chrome.length)
          SamplerSlot(
            label: chrome[i].label,
            durationMs: chrome[i].durationMs,
            path: chrome[i].path,
          )
        else
          const SamplerSlot(),
    ];
  }

  SamplerPlayMode? _playModeFromWire(String? playMode) => switch (playMode) {
    kSamplerPlayModeOneshot => SamplerPlayMode.oneshot,
    kSamplerPlayModeHold => SamplerPlayMode.hold,
    kSamplerPlayModeLoop => SamplerPlayMode.loop,
    _ => null,
  };

  @override
  Widget build(BuildContext context) {
    final trackId = ref.watch(deckTrackIdProvider(widget.deckId));
    if (trackId != _hydratedTrackId) {
      _hydratedTrackId = trackId;
      final library = ref.read(libraryTransportProvider).asData?.value;
      if (library != null && trackId != null && trackId.isNotEmpty) {
        unawaited(library.refreshTrack(trackId: trackId));
      }
    }
    final padMode = ref.watch(deckPadModeProvider(widget.deckId));
    final hotCues = ref.watch(deckHotCuesProvider(widget.deckId));
    final rustBanks = ref.watch(samplerBanksProvider).asData?.value ?? const [];
    final banks = [
      for (final bank in rustBanks)
        SamplerBank(
          id: bank.id,
          name: bank.name,
          playMode: switch (bank.playMode) {
            SamplerPlayMode.oneshot => kSamplerPlayModeOneshot,
            SamplerPlayMode.hold => kSamplerPlayModeHold,
            SamplerPlayMode.loop => kSamplerPlayModeLoop,
            null => null,
          },
        ),
    ];
    final activeBankId = ref.watch(
      deckActiveSamplerBankIdProvider(widget.deckId),
    );
    final resolvedBankId =
        activeBankId != null && banks.any((b) => b.id == activeBankId)
        ? activeBankId
        : (banks.isNotEmpty ? banks.first.id : null);
    final slots = _slotsFromChrome(
      ref.watch(deckSamplerSlotsProvider(widget.deckId)),
    );
    final keyShiftSemitones = ref
        .watch(deckKeyShiftProvider(widget.deckId))
        .round();
    final keyboardPage = ref.watch(deckKeyboardPageProvider(widget.deckId));
    final keyShiftPage = ref.watch(deckKeyShiftPageProvider(widget.deckId));
    final keyboardRootHotCue = ref.watch(
      deckKeyboardRootProvider(widget.deckId),
    );

    return DeckPadsPanel(
      padMode: padMode,
      onPadMode: (mode) {
        unawaited(
          _run(
            (engine) => engine.setPadMode(
              deckId: widget.deckId,
              mode: _toEnginePadMode(mode),
            ),
          ),
        );
      },
      hotCues: hotCues,
      onHotCuePress: (slot, shift) {
        unawaited(
          _run(
            (engine) => engine.hotCuePadPress(
              deckId: widget.deckId,
              slot: slot,
              shift: shift,
            ),
          ),
        );
      },
      onHotCueRelease: (slot) {
        unawaited(
          _run(
            (engine) =>
                engine.hotCuePadRelease(deckId: widget.deckId, slot: slot),
          ),
        );
      },
      onLoopRollPress: (slot) {
        unawaited(
          _run(
            (engine) =>
                engine.loopRollPadPress(deckId: widget.deckId, slot: slot),
          ),
        );
      },
      onLoopRollRelease: (slot) {
        unawaited(
          _run(
            (engine) =>
                engine.loopRollPadRelease(deckId: widget.deckId, slot: slot),
          ),
        );
      },
      onBeatJumpPress: (slot) {
        unawaited(
          _run(
            (engine) =>
                engine.beatJumpPadPress(deckId: widget.deckId, slot: slot),
          ),
        );
      },
      onBeatJumpRelease: (slot) {
        unawaited(
          _run(
            (engine) =>
                engine.beatJumpPadRelease(deckId: widget.deckId, slot: slot),
          ),
        );
      },
      samplerSlots: slots,
      samplerBanks: banks,
      activeBankId: resolvedBankId,
      onSamplerPress: (slot, shift) {
        if (shift) {
          unawaited(
            _run(
              (engine) =>
                  engine.clearSampler(deckId: widget.deckId, slot: slot),
            ),
          );
          return;
        }
        unawaited(
          _run(
            (engine) => engine.samplerPadPress(
              deckId: widget.deckId,
              slot: slot,
              shift: false,
            ),
          ),
        );
      },
      onSamplerRelease: (slot) {
        unawaited(
          _run(
            (engine) =>
                engine.samplerPadRelease(deckId: widget.deckId, slot: slot),
          ),
        );
      },
      onSelectBank: (id) {
        unawaited(
          _run(
            (engine) =>
                engine.setSamplerBank(deckId: widget.deckId, bankId: id),
          ),
        );
      },
      onSaveBank: (bankId, name, playMode) {
        unawaited(_saveBank(bankId, name, playMode));
      },
      onSamplerAssign: (slot, payload) {
        unawaited(_assignSampler(slot, payload));
      },
      stemMute: ref.watch(deckStemMuteProvider(widget.deckId)),
      stemIsolate: ref.watch(deckStemIsolateProvider(widget.deckId)),
      stemsReady: ref.watch(deckStemsReadyProvider(widget.deckId)),
      stemsGenerating: ref.watch(deckStemsGeneratingProvider(widget.deckId)),
      onStemsPress: (slot) {
        unawaited(
          _run(
            (engine) => engine.padPress(
              deckId: widget.deckId,
              slot: slot,
              shift: false,
            ),
          ),
        );
      },
      keyShiftSemitones: keyShiftSemitones,
      keyboardPage: keyboardPage,
      keyShiftPage: keyShiftPage,
      keyboardRootHotCue: keyboardRootHotCue,
      onSelectRoot: (slot) {
        unawaited(
          _run(
            (engine) =>
                engine.setKeyboardRoot(deckId: widget.deckId, slot: slot),
          ),
        );
      },
      onPrevPage: () => _stepPage(-1, padMode, keyboardPage, keyShiftPage),
      onNextPage: () => _stepPage(1, padMode, keyboardPage, keyShiftPage),
      onKeyShiftPress: (slot) {
        unawaited(
          _run(
            (engine) => engine.keyShiftPadPress(
              deckId: widget.deckId,
              slot: slot,
              shift: shiftKeyPressed(),
            ),
          ),
        );
      },
      onKeyboardPress: (slot) {
        unawaited(
          _run(
            (engine) => engine.keyboardPadPress(
              deckId: widget.deckId,
              slot: slot,
              shift: shiftKeyPressed(),
            ),
          ),
        );
      },
      onKeyboardRelease: (slot) {
        unawaited(
          _run(
            (engine) =>
                engine.keyboardPadRelease(deckId: widget.deckId, slot: slot),
          ),
        );
      },
      hasTrack: widget.hasTrack,
      disabled: widget.disabled,
      bordered: widget.bordered,
    );
  }

  Future<void> _saveBank(String bankId, String name, String? playMode) async {
    final ok = await _run(
      (engine) => engine.updateSamplerBank(
        bankId: bankId,
        name: name,
        playMode: _playModeFromWire(playMode),
      ),
    );
    if (ok) {
      ref.invalidate(samplerBanksProvider);
    }
  }

  Future<void> _assignSampler(int slot, TrackDragPayload payload) async {
    await _run((engine) async {
      if (payload.trackId != null && payload.trackId!.isNotEmpty) {
        await engine.assignSamplerTrack(
          deckId: widget.deckId,
          slot: slot,
          trackId: payload.trackId!,
        );
      } else {
        await engine.assignSampler(
          deckId: widget.deckId,
          slot: slot,
          path: payload.path,
        );
      }
    });
  }
}
