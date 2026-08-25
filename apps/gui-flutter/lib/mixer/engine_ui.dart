import 'package:gui_flutter/mixer/level_meter.dart';
import 'package:gui_flutter/mixer/pad_modes.dart';
import 'package:gui_flutter/mixer/tempo_format.dart';
import 'package:gui_flutter/src/rust/api/engine.dart' hide PadMode;

class MixerChannelUi {
  const MixerChannelUi({
    this.volume = 1.0,
    this.eqLow = 0.5,
    this.eqMid = 0.5,
    this.eqHigh = 0.5,
    this.filter = 0.5,
    this.gainTrim = 0.5,
    this.headphoneCue = false,
  });

  static const defaults = MixerChannelUi();

  final double volume;
  final double eqLow;
  final double eqMid;
  final double eqHigh;
  final double filter;
  final double gainTrim;
  final bool headphoneCue;

  MixerChannelUi patchedFrom(EngineEvt evt) => MixerChannelUi(
    volume: evt.volume ?? volume,
    eqLow: evt.eqLow ?? eqLow,
    eqMid: evt.eqMid ?? eqMid,
    eqHigh: evt.eqHigh ?? eqHigh,
    filter: evt.filter ?? filter,
    gainTrim: evt.gainTrim ?? gainTrim,
    headphoneCue: evt.headphoneCue ?? headphoneCue,
  );

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is MixerChannelUi &&
          volume == other.volume &&
          eqLow == other.eqLow &&
          eqMid == other.eqMid &&
          eqHigh == other.eqHigh &&
          filter == other.filter &&
          gainTrim == other.gainTrim &&
          headphoneCue == other.headphoneCue;

  @override
  int get hashCode =>
      Object.hash(volume, eqLow, eqMid, eqHigh, filter, gainTrim, headphoneCue);
}

class EngineUiSnapshot {
  const EngineUiSnapshot({
    required this.running,
    required this.titles,
    this.playing = const {},
    this.channels = const {},
    this.levels = const {},
    this.crossfader = 0.5,
    this.trackIds = const {},
    this.durationMs = const {},
    this.speeds = const {},
    this.tempoRanges = const {},
    this.padModes = const {},
    this.syncModes = const {},
    this.activeLoops = const {},
    this.quantize = const {},
    this.jogTouching = const {},
    this.loudnessLufs = const {},
    this.autoGainDb = const {},
    this.activeSamplerBankIds = const {},
    this.samplerSlots = const {},
    this.masterDeck = 0,
    this.cueMix = 0.0,
    this.masterCue = false,
  });

  static const empty = EngineUiSnapshot(running: false, titles: {});

  final bool running;
  final Map<int, String> titles;
  final Map<int, bool> playing;
  final Map<int, MixerChannelUi> channels;
  final Map<int, DeckLevels> levels;
  final double crossfader;
  final double cueMix;
  final bool masterCue;
  final Map<int, String> trackIds;
  final Map<int, int> durationMs;
  final Map<int, double> speeds;
  final Map<int, double> tempoRanges;
  final Map<int, PadMode> padModes;
  final Map<int, SyncMode> syncModes;
  final Map<int, ActiveLoopInfo> activeLoops;
  final Map<int, bool> quantize;
  final Map<int, bool> jogTouching;
  final Map<int, double> loudnessLufs;
  final Map<int, double> autoGainDb;
  final Map<int, String> activeSamplerBankIds;
  final Map<int, List<SamplerSlotChrome>> samplerSlots;
  final int masterDeck;

  String? titleFor(int deckId) => titles[deckId];

  bool isPlaying(int deckId) => playing[deckId] ?? false;

  String? trackIdFor(int deckId) => trackIds[deckId];

  int? durationMsFor(int deckId) => durationMs[deckId];

  double speedFor(int deckId) => speeds[deckId] ?? 0.5;

  double tempoRangeFor(int deckId) => tempoRanges[deckId] ?? kDefaultTempoRange;

  PadMode padModeFor(int deckId) => padModes[deckId] ?? PadMode.hotCue;

  SyncMode syncModeFor(int deckId) => syncModes[deckId] ?? SyncMode.off;

  ActiveLoopInfo? activeLoopFor(int deckId) => activeLoops[deckId];

  bool quantizeFor(int deckId) => quantize[deckId] ?? true;

  bool jogTouchingFor(int deckId) => jogTouching[deckId] ?? false;

  double? loudnessLufsFor(int deckId) => loudnessLufs[deckId];

  double autoGainDbFor(int deckId) => autoGainDb[deckId] ?? 0;

  String? activeSamplerBankIdFor(int deckId) => activeSamplerBankIds[deckId];

  List<SamplerSlotChrome> samplerSlotsFor(int deckId) =>
      samplerSlots[deckId] ?? const [];

  bool isMaster(int deckId) => masterDeck == deckId;

  MixerChannelUi channelFor(int deckId) =>
      channels[deckId] ?? MixerChannelUi.defaults;

  DeckLevels levelsFor(int deckId) => levels[deckId] ?? zeroDeckLevels;

  EngineUiSnapshot copyWith({
    bool? running,
    Map<int, String>? titles,
    Map<int, bool>? playing,
    Map<int, MixerChannelUi>? channels,
    Map<int, DeckLevels>? levels,
    double? crossfader,
    Map<int, String>? trackIds,
    Map<int, int>? durationMs,
    Map<int, double>? speeds,
    Map<int, double>? tempoRanges,
    Map<int, PadMode>? padModes,
    Map<int, SyncMode>? syncModes,
    Map<int, ActiveLoopInfo>? activeLoops,
    Map<int, bool>? quantize,
    Map<int, bool>? jogTouching,
    Map<int, double>? loudnessLufs,
    Map<int, double>? autoGainDb,
    Map<int, String>? activeSamplerBankIds,
    Map<int, List<SamplerSlotChrome>>? samplerSlots,
    int? masterDeck,
    double? cueMix,
    bool? masterCue,
  }) => EngineUiSnapshot(
    running: running ?? this.running,
    titles: titles ?? this.titles,
    playing: playing ?? this.playing,
    channels: channels ?? this.channels,
    levels: levels ?? this.levels,
    crossfader: crossfader ?? this.crossfader,
    trackIds: trackIds ?? this.trackIds,
    durationMs: durationMs ?? this.durationMs,
    speeds: speeds ?? this.speeds,
    tempoRanges: tempoRanges ?? this.tempoRanges,
    padModes: padModes ?? this.padModes,
    syncModes: syncModes ?? this.syncModes,
    activeLoops: activeLoops ?? this.activeLoops,
    quantize: quantize ?? this.quantize,
    jogTouching: jogTouching ?? this.jogTouching,
    loudnessLufs: loudnessLufs ?? this.loudnessLufs,
    autoGainDb: autoGainDb ?? this.autoGainDb,
    activeSamplerBankIds: activeSamplerBankIds ?? this.activeSamplerBankIds,
    samplerSlots: samplerSlots ?? this.samplerSlots,
    masterDeck: masterDeck ?? this.masterDeck,
    cueMix: cueMix ?? this.cueMix,
    masterCue: masterCue ?? this.masterCue,
  );
}

EngineUiSnapshot applyEngineEvt(EngineUiSnapshot prev, EngineEvt evt) {
  switch (evt.kind) {
    case EngineEvtKind.status:
      return prev.copyWith(
        running: evt.running ?? prev.running,
        crossfader: evt.crossfader ?? prev.crossfader,
        masterDeck: evt.masterDeck ?? prev.masterDeck,
        cueMix: evt.cueMix ?? prev.cueMix,
        masterCue: evt.masterCue ?? prev.masterCue,
      );
    case EngineEvtKind.updated:
      final id = evt.deckId;
      if (id == null) {
        return prev;
      }
      final unloaded = evt.durationKnown && evt.durationMs == null;
      final nextTitles = Map<int, String>.from(prev.titles);
      final title = evt.track;
      // Engine snapshots omit library metadata (`track`/`title` are always
      // null). Keep the host title from load; only replace when the evt
      // actually carries one. Unload (duration_ms → null) clears host identity.
      if (unloaded) {
        nextTitles.remove(id);
      } else if (title != null && title.isNotEmpty) {
        nextTitles[id] = title;
      }
      final nextPlaying = Map<int, bool>.from(prev.playing);
      if (evt.playing != null) {
        nextPlaying[id] = evt.playing!;
      }
      final nextChannels = Map<int, MixerChannelUi>.from(prev.channels);
      nextChannels[id] = prev.channelFor(id).patchedFrom(evt);
      final nextTrackIds = Map<int, String>.from(prev.trackIds);
      if (unloaded) {
        nextTrackIds.remove(id);
      } else if (evt.trackId != null && evt.trackId!.isNotEmpty) {
        nextTrackIds[id] = evt.trackId!;
      }
      final nextDurations = Map<int, int>.from(prev.durationMs);
      if (unloaded) {
        nextDurations.remove(id);
      } else if (evt.durationMs != null) {
        nextDurations[id] = evt.durationMs!;
      }
      final nextSpeeds = Map<int, double>.from(prev.speeds);
      if (evt.speed != null) {
        nextSpeeds[id] = evt.speed!;
      }
      final nextRanges = Map<int, double>.from(prev.tempoRanges);
      if (evt.tempoRange != null) {
        nextRanges[id] = evt.tempoRange!;
      }
      final nextPadModes = Map<int, PadMode>.from(prev.padModes);
      final enginePadMode = evt.padMode;
      if (enginePadMode != null) {
        nextPadModes[id] = enginePadMode;
      }
      final nextSyncModes = Map<int, SyncMode>.from(prev.syncModes);
      if (evt.syncMode != null) {
        nextSyncModes[id] = evt.syncMode!;
      }
      final nextActiveLoops = Map<int, ActiveLoopInfo>.from(prev.activeLoops);
      if (evt.activeLoopKnown) {
        final region = evt.activeLoop;
        if (region != null && region.active) {
          nextActiveLoops[id] = region;
        } else {
          nextActiveLoops.remove(id);
        }
      }
      if (unloaded) {
        nextActiveLoops.remove(id);
      }
      final nextQuantize = Map<int, bool>.from(prev.quantize);
      if (evt.quantize != null) {
        nextQuantize[id] = evt.quantize!;
      }
      final nextJog = Map<int, bool>.from(prev.jogTouching);
      if (unloaded) {
        nextJog[id] = false;
      } else if (evt.jogTouching != null) {
        nextJog[id] = evt.jogTouching!;
      }
      final nextLoudness = Map<int, double>.from(prev.loudnessLufs);
      final nextAutoGain = Map<int, double>.from(prev.autoGainDb);
      final nextSamplerBanks = Map<int, String>.from(prev.activeSamplerBankIds);
      final nextSamplerSlots = Map<int, List<SamplerSlotChrome>>.from(
        prev.samplerSlots,
      );
      if (unloaded) {
        nextLoudness.remove(id);
        nextAutoGain.remove(id);
        nextSamplerBanks.remove(id);
        nextSamplerSlots.remove(id);
      } else {
        if (evt.loudnessLufs != null) {
          nextLoudness[id] = evt.loudnessLufs!;
        }
        if (evt.autoGainDb != null) {
          nextAutoGain[id] = evt.autoGainDb!;
        }
        if (evt.activeSamplerBankIdKnown) {
          final bankId = evt.activeSamplerBankId;
          if (bankId != null && bankId.isNotEmpty) {
            nextSamplerBanks[id] = bankId;
          } else {
            nextSamplerBanks.remove(id);
          }
        }
        if (evt.samplerSlotsKnown) {
          nextSamplerSlots[id] = List<SamplerSlotChrome>.from(
            evt.samplerSlots ?? const [],
          );
        }
      }
      return prev.copyWith(
        titles: nextTitles,
        playing: nextPlaying,
        channels: nextChannels,
        trackIds: nextTrackIds,
        durationMs: nextDurations,
        speeds: nextSpeeds,
        tempoRanges: nextRanges,
        padModes: nextPadModes,
        syncModes: nextSyncModes,
        activeLoops: nextActiveLoops,
        quantize: nextQuantize,
        jogTouching: nextJog,
        loudnessLufs: nextLoudness,
        autoGainDb: nextAutoGain,
        activeSamplerBankIds: nextSamplerBanks,
        samplerSlots: nextSamplerSlots,
      );
    case EngineEvtKind.levels:
      final id = evt.deckId;
      if (id == null || evt.peakL == null || evt.peakR == null) {
        return prev;
      }
      final nextLevels = Map<int, DeckLevels>.from(prev.levels);
      final prevLevels = prev.levelsFor(id);
      nextLevels[id] = DeckLevels(
        peakL: evt.peakL!,
        peakR: evt.peakR!,
        peakHoldL: evt.peakHoldL ?? prevLevels.peakHoldL,
        peakHoldR: evt.peakHoldR ?? prevLevels.peakHoldR,
      );
      return prev.copyWith(levels: nextLevels);
    case EngineEvtKind.position:
    case EngineEvtKind.error:
    case EngineEvtKind.notice:
      return prev;
  }
}
