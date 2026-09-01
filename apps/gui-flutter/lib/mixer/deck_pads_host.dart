import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/mixer/deck_pads_panel.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/mixer/pad_modes.dart';
import 'package:gui_flutter/mixer/pads/sampler_pads.dart';
import 'package:gui_flutter/mixer/track_drag.dart';
import 'package:gui_flutter/settings/settings_providers.dart';
import 'package:gui_flutter/src/rust/api/engine.dart' as rust;
import 'package:gui_flutter/src/rust/api/library.dart';

/// Watches engine/library providers and publishes named pad press/release cmds.
class DeckPadsHost extends ConsumerStatefulWidget {
  const DeckPadsHost({
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
    showFToast(context: context, variant: .destructive, title: Text('$e'));
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
