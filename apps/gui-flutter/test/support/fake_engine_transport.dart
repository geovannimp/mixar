// Test doubles for EngineTransport (FRB opaque). Hand-maintained stubs.
import 'dart:async';

import 'package:gui_flutter/src/rust/api/engine.dart';
import 'package:gui_flutter/src/rust/api/library.dart' show SamplerPlayMode;
import 'package:gui_flutter/src/rust/api/settings.dart' show JogModeSetting;

/// Minimal [EngineTransport] fake that records seek / jog calls.
class FakeEngineTransport implements EngineTransport {
  final StreamController<EngineEvt> _events =
      StreamController<EngineEvt>.broadcast();

  final List<(int deckId, int positionMs)> seeks = [];
  final List<(int deckId, int delta)> jogTurns = [];
  final List<(int deckId, bool touching)> jogTouches = [];
  final List<double> masterVolumes = [];
  final List<double> cueVolumes = [];
  final List<double> cueMixes = [];
  final List<(int slot, FxKind kind)> fxUnitSlots = [];
  final List<(int slot, bool enabled)> fxUnitSlotEnabled = [];
  final List<(int slot, double wet)> fxUnitWets = [];
  final List<(int slot, int param, double value)> fxUnitParams = [];
  final List<(int deckId, bool routed)> deckFxRouted = [];
  final List<(int deckId, PadMode mode)> padModes = [];
  final List<(int deckId, int slot)> loopRollPresses = [];

  @override
  void dispose() {
    unawaited(_events.close());
  }

  @override
  bool get isDisposed => false;

  @override
  Future<void> seek({required int deckId, required int positionMs}) async {
    seeks.add((deckId, positionMs));
  }

  @override
  Future<void> jogTurn({required int deckId, required int delta}) async {
    jogTurns.add((deckId, delta));
  }

  @override
  Future<void> jogTouch({required int deckId, required bool touching}) async {
    jogTouches.add((deckId, touching));
  }

  @override
  Future<void> assignSampler({
    required int deckId,
    required int slot,
    required String path,
  }) async {}

  @override
  Future<void> assignSamplerTrack({
    required int deckId,
    required int slot,
    required String trackId,
  }) async {}

  @override
  Future<void> beatJumpPadPress({
    required int deckId,
    required int slot,
  }) async {}

  @override
  Future<void> beatJumpPadRelease({
    required int deckId,
    required int slot,
  }) async {}

  @override
  Future<void> beginCueHold({required int deckId}) async {}

  @override
  Future<EngineBusHandle> buses() async => throw UnimplementedError('buses');

  @override
  Future<void> clearSampler({required int deckId, required int slot}) async {}

  @override
  Future<void> endCueHold({required int deckId}) async {}

  @override
  Future<void> exitLoop({required int deckId}) async {}

  @override
  Future<void> hotCuePadPress({
    required int deckId,
    required int slot,
    required bool shift,
  }) async {}

  @override
  Future<void> hotCuePadRelease({
    required int deckId,
    required int slot,
  }) async {}

  @override
  Future<bool> isRunning() async => false;

  @override
  Future<void> keyShiftPadPress({
    required int deckId,
    required int slot,
    required bool shift,
  }) async {}

  @override
  Future<void> keyShiftPadRelease({
    required int deckId,
    required int slot,
  }) async {}

  @override
  Future<void> keyboardPadPress({
    required int deckId,
    required int slot,
    required bool shift,
  }) async {}

  @override
  Future<void> keyboardPadRelease({
    required int deckId,
    required int slot,
  }) async {}

  @override
  Future<void> loadLibraryTrack({
    required int deckId,
    required String trackId,
  }) async {}

  @override
  Future<void> loadPath({required int deckId, required String path}) async {}

  @override
  Future<void> loopIn({required int deckId, required int positionMs}) async {}

  @override
  Future<void> loopOut({required int deckId, required int positionMs}) async {}

  @override
  Future<void> loopRollPadPress({
    required int deckId,
    required int slot,
  }) async {
    loopRollPresses.add((deckId, slot));
  }

  @override
  Future<void> loopRollPadRelease({
    required int deckId,
    required int slot,
  }) async {}

  @override
  Future<void> padPress({
    required int deckId,
    required int slot,
    required bool shift,
  }) async {}

  @override
  Future<void> padRelease({required int deckId, required int slot}) async {}

  @override
  Future<void> pause({required int deckId}) async {}

  @override
  Future<void> play({required int deckId}) async {}

  @override
  Future<void> recallSavedLoop({
    required int deckId,
    required int inMs,
    required int outMs,
  }) async {}

  @override
  Future<void> restartFromSettings() async {}

  @override
  Future<void> samplerPadPress({
    required int deckId,
    required int slot,
    required bool shift,
  }) async {}

  @override
  Future<void> samplerPadRelease({
    required int deckId,
    required int slot,
  }) async {}

  @override
  Future<void> setAutoLoop({
    required int deckId,
    required double beats,
  }) async {}

  @override
  Future<void> setCrossfader({required double position}) async {}

  @override
  Future<void> setCrossfaderAssign({
    required int deckId,
    required CrossfaderAssign assignment,
  }) async {}

  @override
  Future<void> setCueMix({required double mix}) async {
    cueMixes.add(mix);
  }

  @override
  Future<void> setCueVolume({required double volume}) async {
    cueVolumes.add(volume);
  }

  @override
  Future<void> setCuePoint({required int deckId}) async {}

  @override
  Future<void> setEqBand({
    required int deckId,
    required EqBand band,
    required double gain,
  }) async {}

  @override
  Future<void> setFilter({required int deckId, required double filter}) async {}

  @override
  Future<void> setDeckFxRouted({
    required int deckId,
    required bool routed,
  }) async {
    deckFxRouted.add((deckId, routed));
  }

  @override
  Future<void> setFxUnitParam({
    required int slot,
    required int param,
    required double value,
  }) async {
    fxUnitParams.add((slot, param, value));
  }

  @override
  Future<void> setFxUnitSlot({required int slot, required FxKind kind}) async {
    fxUnitSlots.add((slot, kind));
  }

  @override
  Future<void> setFxUnitSlotEnabled({
    required int slot,
    required bool enabled,
  }) async {
    fxUnitSlotEnabled.add((slot, enabled));
  }

  @override
  Future<void> setFxUnitWet({required int slot, required double wet}) async {
    fxUnitWets.add((slot, wet));
  }

  @override
  Future<void> setGainTrim({
    required int deckId,
    required double gainTrim,
  }) async {}

  @override
  Future<void> setHeadphoneCue({
    required int deckId,
    required bool enabled,
  }) async {}

  @override
  Future<void> setJogMode({
    required int deckId,
    required JogModeSetting top,
    required JogModeSetting outer,
  }) async {}

  @override
  Future<void> setKeyLock({required int deckId, required bool enabled}) async {}

  @override
  Future<void> setKeyShift({
    required int deckId,
    required double semitones,
  }) async {}

  @override
  Future<void> setKeyShiftPage({
    required int deckId,
    required int page,
  }) async {}

  @override
  Future<void> setKeyboardPage({
    required int deckId,
    required int page,
  }) async {}

  @override
  Future<void> setKeyboardRoot({
    required int deckId,
    required int slot,
  }) async {}

  @override
  Future<void> setMasterCue({required bool enabled}) async {}

  @override
  Future<void> setMasterDeck({required int deckId}) async {}

  @override
  Future<void> setMasterVolume({required double volume}) async {
    masterVolumes.add(volume);
  }

  @override
  Future<void> setPadMode({required int deckId, required PadMode mode}) async {
    padModes.add((deckId, mode));
    _events.add(
      EngineEvt(kind: EngineEvtKind.updated, deckId: deckId, padMode: mode),
    );
  }

  @override
  Future<void> setQuantize({
    required int deckId,
    required bool enabled,
  }) async {}

  @override
  Future<void> setSamplerBank({
    required int deckId,
    required String bankId,
  }) async {}

  @override
  Future<void> setSlip({required int deckId, required bool enabled}) async {}

  @override
  Future<void> setSpeed({required int deckId, required double speed}) async {}

  @override
  Future<void> setTempoRange({
    required int deckId,
    required double tempoRange,
  }) async {}

  @override
  Future<void> setVolume({required int deckId, required double volume}) async {}

  @override
  Future<void> stop() async {}

  @override
  Stream<EngineEvt> subscribeEvents() => _events.stream;

  @override
  Future<void> toggleSync({
    required int deckId,
    required bool beatSync,
  }) async {}

  @override
  Future<void> unload({required int deckId}) async {}

  @override
  Future<void> updateSamplerBank({
    required String bankId,
    required String name,
    SamplerPlayMode? playMode,
  }) async {}
}
