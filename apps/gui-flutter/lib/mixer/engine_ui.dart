import 'package:gui_flutter/mixer/level_meter.dart';
import 'package:gui_flutter/mixer/pad_modes.dart';
import 'package:gui_flutter/mixer/tempo_format.dart';
import 'package:gui_flutter/src/rust/api/engine.dart' as rust;

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

  MixerChannelUi patchedFrom(rust.EngineEvt evt) => MixerChannelUi(
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
  });

  static const empty = EngineUiSnapshot(running: false, titles: {});

  final bool running;
  final Map<int, String> titles;
  final Map<int, bool> playing;
  final Map<int, MixerChannelUi> channels;
  final Map<int, DeckLevels> levels;
  final double crossfader;
  final Map<int, String> trackIds;
  final Map<int, int> durationMs;
  final Map<int, double> speeds;
  final Map<int, double> tempoRanges;
  final Map<int, PadMode> padModes;

  String? titleFor(int deckId) => titles[deckId];

  bool isPlaying(int deckId) => playing[deckId] ?? false;

  String? trackIdFor(int deckId) => trackIds[deckId];

  int? durationMsFor(int deckId) => durationMs[deckId];

  double speedFor(int deckId) => speeds[deckId] ?? 0.5;

  double tempoRangeFor(int deckId) => tempoRanges[deckId] ?? kDefaultTempoRange;

  PadMode padModeFor(int deckId) => padModes[deckId] ?? PadMode.hotCue;

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
  );
}

EngineUiSnapshot applyEngineEvt(EngineUiSnapshot prev, rust.EngineEvt evt) {
  switch (evt.kind) {
    case rust.EngineEvtKind.status:
      return prev.copyWith(
        running: evt.running ?? prev.running,
        crossfader: evt.crossfader ?? prev.crossfader,
      );
    case rust.EngineEvtKind.updated:
      final id = evt.deckId;
      if (id == null) {
        return prev;
      }
      final nextTitles = Map<int, String>.from(prev.titles);
      final title = evt.track;
      // Engine snapshots omit library metadata (`track`/`title` are always
      // null). Keep the host title from load; only replace when the evt
      // actually carries one.
      if (title != null && title.isNotEmpty) {
        nextTitles[id] = title;
      }
      final nextPlaying = Map<int, bool>.from(prev.playing);
      if (evt.playing != null) {
        nextPlaying[id] = evt.playing!;
      }
      final nextChannels = Map<int, MixerChannelUi>.from(prev.channels);
      nextChannels[id] = prev.channelFor(id).patchedFrom(evt);
      final nextTrackIds = Map<int, String>.from(prev.trackIds);
      if (evt.trackId != null && evt.trackId!.isNotEmpty) {
        nextTrackIds[id] = evt.trackId!;
      }
      final nextDurations = Map<int, int>.from(prev.durationMs);
      if (evt.durationMs != null) {
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
        nextPadModes[id] = _uiPadMode(enginePadMode);
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
      );
    case rust.EngineEvtKind.levels:
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
    case rust.EngineEvtKind.position:
    case rust.EngineEvtKind.error:
    case rust.EngineEvtKind.notice:
      return prev;
  }
}

PadMode _uiPadMode(rust.PadMode engine) => switch (engine) {
  rust.PadMode.hotCue => PadMode.hotCue,
  rust.PadMode.loopRoll => PadMode.loopRoll,
  rust.PadMode.beatJump => PadMode.beatJump,
  rust.PadMode.sampler => PadMode.sampler,
};
