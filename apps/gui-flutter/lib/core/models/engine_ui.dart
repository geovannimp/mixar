import 'package:flutter/foundation.dart';
import 'package:gui_flutter/core/models/bus_event_payload.dart';
import 'package:gui_flutter/core/models/pad_modes.dart';
import 'package:gui_flutter/core/utils/tempo_format.dart';
import 'package:gui_flutter/src/rust/api/event_mirrors.dart' as bus;
import 'package:gui_flutter/src/rust/api/event_mirrors.dart'
    show
        CrossfaderAssign,
        FxSlotState,
        LoopRegion,
        PadMode,
        SamplerSlotInfo,
        SyncMode;
import 'package:gui_flutter/widgets/mixer/level_meter.dart';

class MixerChannelUi {
  const new({
    this.volume = 1.0,
    this.eqLow = 0.5,
    this.eqMid = 0.5,
    this.eqHigh = 0.5,
    this.filter = 0.5,
    this.gainTrim = 0.5,
    this.fxRouted = false,
    this.headphoneCue = false,
    this.crossfaderAssign = CrossfaderAssign.a,
  });

  static const defaults = MixerChannelUi();

  final double volume;
  final double eqLow;
  final double eqMid;
  final double eqHigh;
  final double filter;
  final double gainTrim;
  final bool fxRouted;
  final bool headphoneCue;
  final CrossfaderAssign crossfaderAssign;

  /// Channel state from a full deck snapshot.
  factory MixerChannelUi.fromDeckUpdate(bus.DeckUpdated update) =>
      MixerChannelUi(
        volume: update.volume,
        eqLow: update.eq.low,
        eqMid: update.eq.mid,
        eqHigh: update.eq.high,
        filter: update.filter,
        gainTrim: update.gainTrim,
        fxRouted: update.fxRouted,
        headphoneCue: update.headphoneCue,
        crossfaderAssign: update.crossfaderAssign,
      );

  /// Optimistic write for the FX routing switch (engine evts supersede it).
  MixerChannelUi withFxRouted(bool value) => MixerChannelUi(
    volume: volume,
    eqLow: eqLow,
    eqMid: eqMid,
    eqHigh: eqHigh,
    filter: filter,
    gainTrim: gainTrim,
    fxRouted: value,
    headphoneCue: headphoneCue,
    crossfaderAssign: crossfaderAssign,
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
          fxRouted == other.fxRouted &&
          headphoneCue == other.headphoneCue &&
          crossfaderAssign == other.crossfaderAssign;

  @override
  int get hashCode => Object.hash(
    volume,
    eqLow,
    eqMid,
    eqHigh,
    filter,
    gainTrim,
    fxRouted,
    headphoneCue,
    crossfaderAssign,
  );
}

class EngineUiSnapshot {
  const new({
    required this.running,
    required this.trackPaths,
    this.playing = const {},
    this.channels = const {},
    this.levels = const {},
    this.masterLevels = zeroDeckLevels,
    this.crossfader = 0.5,
    this.trackIds = const {},
    this.durationMs = const {},
    this.speeds = const {},
    this.tempoRanges = const {},
    this.keyLocks = const {},
    this.keyShifts = const {},
    this.keyboardPages = const {},
    this.keyShiftPages = const {},
    this.keyboardRoots = const {},
    this.padModes = const {},
    this.stemsReady = const {},
    this.stemsLive = const {},
    this.stemMute = const {},
    this.stemIsolate = const {},
    this.syncModes = const {},
    this.activeLoops = const {},
    this.pendingLoopInMs = const {},
    this.quantize = const {},
    this.slipEnabled = const {},
    this.jogTouching = const {},
    this.loudnessLufs = const {},
    this.autoGainDb = const {},
    this.activeSamplerBankIds = const {},
    this.samplerSlots = const {},
    this.fxSlots = const [],
    this.masterDeck = 0,
    this.masterVolume = 1.0,
    this.cueVolume = 1.0,
    this.cueMix = 0.5,
    this.masterCue = false,
  });

  static const empty = EngineUiSnapshot(running: false, trackPaths: {});

  final bool running;
  final Map<int, String> trackPaths;
  final Map<int, bool> playing;
  final Map<int, MixerChannelUi> channels;
  final Map<int, DeckLevels> levels;
  final DeckLevels masterLevels;
  final double crossfader;
  final double masterVolume;
  final double cueVolume;
  final double cueMix;
  final bool masterCue;
  final Map<int, String> trackIds;
  final Map<int, int> durationMs;
  final Map<int, double> speeds;
  final Map<int, double> tempoRanges;
  final Map<int, bool> keyLocks;
  final Map<int, double> keyShifts;
  final Map<int, int> keyboardPages;
  final Map<int, int> keyShiftPages;
  final Map<int, int> keyboardRoots;
  final Map<int, PadMode> padModes;
  final Map<int, bool> stemsReady;
  final Map<int, bool> stemsLive;
  final Map<int, List<bool>> stemMute;
  final Map<int, int> stemIsolate;
  final Map<int, SyncMode> syncModes;
  final Map<int, LoopRegion> activeLoops;
  final Map<int, int> pendingLoopInMs;
  final Map<int, bool> quantize;
  final Map<int, bool> slipEnabled;
  final Map<int, bool> jogTouching;
  final Map<int, double> loudnessLufs;
  final Map<int, double> autoGainDb;
  final Map<int, String> activeSamplerBankIds;
  final Map<int, List<SamplerSlotInfo>> samplerSlots;
  final List<FxSlotState> fxSlots;
  final int masterDeck;

  String? trackPathFor(int deckId) => trackPaths[deckId];

  bool isPlaying(int deckId) => playing[deckId] ?? false;

  String? trackIdFor(int deckId) => trackIds[deckId];

  int? durationMsFor(int deckId) => durationMs[deckId];

  double speedFor(int deckId) => speeds[deckId] ?? 0.5;

  double tempoRangeFor(int deckId) => tempoRanges[deckId] ?? kDefaultTempoRange;

  bool keyLockFor(int deckId) => keyLocks[deckId] ?? false;

  double keyShiftFor(int deckId) => keyShifts[deckId] ?? 0.0;

  int keyboardPageFor(int deckId) => keyboardPages[deckId] ?? kDefaultPitchPage;

  int keyShiftPageFor(int deckId) => keyShiftPages[deckId] ?? kDefaultPitchPage;

  int keyboardRootFor(int deckId) => keyboardRoots[deckId] ?? 0;

  PadMode padModeFor(int deckId) => padModes[deckId] ?? PadMode.hotCue;

  bool stemsReadyFor(int deckId) => stemsReady[deckId] ?? false;

  bool stemsLiveFor(int deckId) => stemsLive[deckId] ?? false;

  List<bool> stemMuteFor(int deckId) =>
      stemMute[deckId] ?? const [false, false, false, false];

  int? stemIsolateFor(int deckId) => stemIsolate[deckId];

  SyncMode syncModeFor(int deckId) => syncModes[deckId] ?? SyncMode.off;

  LoopRegion? activeLoopFor(int deckId) => activeLoops[deckId];

  int? pendingLoopInMsFor(int deckId) => pendingLoopInMs[deckId];

  bool quantizeFor(int deckId) => quantize[deckId] ?? true;

  bool slipEnabledFor(int deckId) => slipEnabled[deckId] ?? false;

  bool jogTouchingFor(int deckId) => jogTouching[deckId] ?? false;

  double? loudnessLufsFor(int deckId) => loudnessLufs[deckId];

  double autoGainDbFor(int deckId) => autoGainDb[deckId] ?? 0;

  String? activeSamplerBankIdFor(int deckId) => activeSamplerBankIds[deckId];

  List<SamplerSlotInfo> samplerSlotsFor(int deckId) =>
      samplerSlots[deckId] ?? const [];

  bool isMaster(int deckId) => masterDeck == deckId;

  MixerChannelUi channelFor(int deckId) =>
      channels[deckId] ?? MixerChannelUi.defaults;

  DeckLevels levelsFor(int deckId) => levels[deckId] ?? zeroDeckLevels;

  EngineUiSnapshot copyWith({
    bool? running,
    Map<int, String>? trackPaths,
    Map<int, bool>? playing,
    Map<int, MixerChannelUi>? channels,
    Map<int, DeckLevels>? levels,
    DeckLevels? masterLevels,
    double? crossfader,
    Map<int, String>? trackIds,
    Map<int, int>? durationMs,
    Map<int, double>? speeds,
    Map<int, double>? tempoRanges,
    Map<int, bool>? keyLocks,
    Map<int, double>? keyShifts,
    Map<int, int>? keyboardPages,
    Map<int, int>? keyShiftPages,
    Map<int, int>? keyboardRoots,
    Map<int, PadMode>? padModes,
    Map<int, bool>? stemsReady,
    Map<int, bool>? stemsLive,
    Map<int, List<bool>>? stemMute,
    Map<int, int>? stemIsolate,
    Map<int, SyncMode>? syncModes,
    Map<int, LoopRegion>? activeLoops,
    Map<int, int>? pendingLoopInMs,
    Map<int, bool>? quantize,
    Map<int, bool>? slipEnabled,
    Map<int, bool>? jogTouching,
    Map<int, double>? loudnessLufs,
    Map<int, double>? autoGainDb,
    Map<int, String>? activeSamplerBankIds,
    Map<int, List<SamplerSlotInfo>>? samplerSlots,
    List<FxSlotState>? fxSlots,
    int? masterDeck,
    double? masterVolume,
    double? cueVolume,
    double? cueMix,
    bool? masterCue,
  }) => EngineUiSnapshot(
    running: running ?? this.running,
    trackPaths: trackPaths ?? this.trackPaths,
    playing: playing ?? this.playing,
    channels: channels ?? this.channels,
    levels: levels ?? this.levels,
    masterLevels: masterLevels ?? this.masterLevels,
    crossfader: crossfader ?? this.crossfader,
    trackIds: trackIds ?? this.trackIds,
    durationMs: durationMs ?? this.durationMs,
    speeds: speeds ?? this.speeds,
    tempoRanges: tempoRanges ?? this.tempoRanges,
    keyLocks: keyLocks ?? this.keyLocks,
    keyShifts: keyShifts ?? this.keyShifts,
    keyboardPages: keyboardPages ?? this.keyboardPages,
    keyShiftPages: keyShiftPages ?? this.keyShiftPages,
    keyboardRoots: keyboardRoots ?? this.keyboardRoots,
    padModes: padModes ?? this.padModes,
    stemsReady: stemsReady ?? this.stemsReady,
    stemsLive: stemsLive ?? this.stemsLive,
    stemMute: stemMute ?? this.stemMute,
    stemIsolate: stemIsolate ?? this.stemIsolate,
    syncModes: syncModes ?? this.syncModes,
    activeLoops: activeLoops ?? this.activeLoops,
    pendingLoopInMs: pendingLoopInMs ?? this.pendingLoopInMs,
    quantize: quantize ?? this.quantize,
    slipEnabled: slipEnabled ?? this.slipEnabled,
    jogTouching: jogTouching ?? this.jogTouching,
    loudnessLufs: loudnessLufs ?? this.loudnessLufs,
    autoGainDb: autoGainDb ?? this.autoGainDb,
    activeSamplerBankIds: activeSamplerBankIds ?? this.activeSamplerBankIds,
    samplerSlots: samplerSlots ?? this.samplerSlots,
    fxSlots: fxSlots ?? this.fxSlots,
    masterDeck: masterDeck ?? this.masterDeck,
    masterVolume: masterVolume ?? this.masterVolume,
    cueVolume: cueVolume ?? this.cueVolume,
    cueMix: cueMix ?? this.cueMix,
    masterCue: masterCue ?? this.masterCue,
  );
}

/// Apply one bus event to the engine mirror.
///
/// `DeckUpdated` carries a complete deck snapshot, so it replaces that deck's
/// state outright — there are no per-field "was it authored" flags to consult.
EngineUiSnapshot applyEngineEvent(EngineUiSnapshot prev, bus.Event event) {
  if (event.deckUpdated case final update?) {
    return _applyDeckUpdate(prev, update);
  }
  if (event.deckLevels case final levels?) {
    return _applyDeckLevels(prev, levels);
  }
  if (event.engineSnapshot case final snapshot?) {
    return _applyEngineSnapshot(prev, snapshot);
  }
  return prev;
}

EngineUiSnapshot _applyDeckUpdate(
  EngineUiSnapshot prev,
  bus.DeckUpdated update,
) {
  final id = update.deck;
  // A null duration is an unloaded deck: track identity and per-deck session
  // state go with it, exactly as the engine cleared them.
  final unloaded = update.durationMs == null;

  final trackPaths = Map<int, String>.from(prev.trackPaths);
  final trackIds = Map<int, String>.from(prev.trackIds);
  final durationMs = Map<int, int>.from(prev.durationMs);
  final playing = Map<int, bool>.from(prev.playing);
  final channels = Map<int, MixerChannelUi>.from(prev.channels);
  final speeds = Map<int, double>.from(prev.speeds);
  final tempoRanges = Map<int, double>.from(prev.tempoRanges);
  final keyLocks = Map<int, bool>.from(prev.keyLocks);
  final keyShifts = Map<int, double>.from(prev.keyShifts);
  final keyboardPages = Map<int, int>.from(prev.keyboardPages);
  final keyShiftPages = Map<int, int>.from(prev.keyShiftPages);
  final keyboardRoots = Map<int, int>.from(prev.keyboardRoots);
  final padModes = Map<int, PadMode>.from(prev.padModes);
  final syncModes = Map<int, SyncMode>.from(prev.syncModes);
  final activeLoops = Map<int, LoopRegion>.from(prev.activeLoops);
  final pendingLoopInMs = Map<int, int>.from(prev.pendingLoopInMs);
  final quantize = Map<int, bool>.from(prev.quantize);
  final slipEnabled = Map<int, bool>.from(prev.slipEnabled);
  final jogTouching = Map<int, bool>.from(prev.jogTouching);
  final loudnessLufs = Map<int, double>.from(prev.loudnessLufs);
  final autoGainDb = Map<int, double>.from(prev.autoGainDb);
  final activeSamplerBankIds = Map<int, String>.from(prev.activeSamplerBankIds);
  final samplerSlots = Map<int, List<SamplerSlotInfo>>.from(prev.samplerSlots);
  final stemsReady = Map<int, bool>.from(prev.stemsReady);
  final stemsLive = Map<int, bool>.from(prev.stemsLive);
  final stemMute = Map<int, List<bool>>.from(prev.stemMute);
  final stemIsolate = Map<int, int>.from(prev.stemIsolate);

  playing[id] = update.playing;
  channels[id] = MixerChannelUi.fromDeckUpdate(update);
  speeds[id] = update.speed;
  tempoRanges[id] = update.tempoRange;
  keyLocks[id] = update.keyLock;
  quantize[id] = update.quantize;
  slipEnabled[id] = update.slipEnabled;
  jogTouching[id] = update.jogTouching;
  padModes[id] = update.padMode;
  syncModes[id] = update.syncMode;

  if (unloaded) {
    trackPaths.remove(id);
    trackIds.remove(id);
    durationMs.remove(id);
    keyShifts.remove(id);
    activeLoops.remove(id);
    pendingLoopInMs.remove(id);
    loudnessLufs.remove(id);
    autoGainDb.remove(id);
    activeSamplerBankIds.remove(id);
    samplerSlots[id] = const [];
    stemsReady.remove(id);
    stemsLive.remove(id);
    stemMute.remove(id);
    stemIsolate.remove(id);
  } else {
    final path = update.track;
    if (path != null && path.isNotEmpty) {
      trackPaths[id] = path;
    }
    final trackId = update.trackId;
    if (trackId != null && trackId.isNotEmpty) {
      trackIds[id] = trackId;
    }
    durationMs[id] = update.durationMs!;
    keyShifts[id] = update.keyShift;
    keyboardPages[id] = update.keyboardPage;
    keyShiftPages[id] = update.keyShiftPage;
    keyboardRoots[id] = update.keyboardRootHotCue;
    final loop = update.activeLoop;
    if (loop != null && loop.active) {
      activeLoops[id] = loop;
    } else {
      activeLoops.remove(id);
    }
    final pending = update.pendingLoopInMs;
    if (pending != null) {
      pendingLoopInMs[id] = pending;
    } else {
      pendingLoopInMs.remove(id);
    }
    if (update.loudnessLufs case final lufs?) {
      loudnessLufs[id] = lufs;
    }
    autoGainDb[id] = update.autoGainDb;
    final bankId = update.activeSamplerBankId;
    if (bankId != null && bankId.isNotEmpty) {
      activeSamplerBankIds[id] = bankId;
    }
    samplerSlots[id] = List<SamplerSlotInfo>.from(update.samplerSlots);
    stemsReady[id] = update.stemsReady;
    stemsLive[id] = update.stemsLive;
    final mute = update.stemMute;
    stemMute[id] = mute.length == 4
        ? List<bool>.from(mute)
        : const [false, false, false, false];
    final isolate = update.stemIsolate;
    if (isolate != null) {
      stemIsolate[id] = isolate;
    } else {
      stemIsolate.remove(id);
    }
  }

  // The FX unit is shared across decks, so any deck's update carries it. Keep
  // the previous list when the payload is unchanged: `FxSlotState ==` compares
  // `params` by identity, so compare fields here.
  final publishedFx = update.fx;
  final fxSlots = _fxSlotsEqual(prev.fxSlots, publishedFx)
      ? prev.fxSlots
      : List<FxSlotState>.from(publishedFx);

  return prev.copyWith(
    trackPaths: trackPaths,
    trackIds: trackIds,
    durationMs: durationMs,
    playing: playing,
    channels: channels,
    speeds: speeds,
    tempoRanges: tempoRanges,
    keyLocks: keyLocks,
    keyShifts: keyShifts,
    keyboardPages: keyboardPages,
    keyShiftPages: keyShiftPages,
    keyboardRoots: keyboardRoots,
    padModes: padModes,
    syncModes: syncModes,
    activeLoops: activeLoops,
    pendingLoopInMs: pendingLoopInMs,
    quantize: quantize,
    slipEnabled: slipEnabled,
    jogTouching: jogTouching,
    loudnessLufs: loudnessLufs,
    autoGainDb: autoGainDb,
    activeSamplerBankIds: activeSamplerBankIds,
    samplerSlots: samplerSlots,
    stemsReady: stemsReady,
    stemsLive: stemsLive,
    stemMute: stemMute,
    stemIsolate: stemIsolate,
    fxSlots: fxSlots,
  );
}

EngineUiSnapshot _applyDeckLevels(
  EngineUiSnapshot prev,
  bus.DeckLevels levels,
) {
  final deck = levels.deck;
  // Holds are authored on every publish, including the engine's zero flush
  // when a deck or the master bus goes idle: apply them, do not 'keep' them.
  if (deck == null) {
    return prev.copyWith(
      masterLevels: DeckLevels(
        peakL: levels.peakL,
        peakR: levels.peakR,
        peakHoldL: levels.peakHoldL,
        peakHoldR: levels.peakHoldR,
      ),
    );
  }
  final nextLevels = Map<int, DeckLevels>.from(prev.levels);
  nextLevels[deck] = DeckLevels(
    peakL: levels.peakL,
    peakR: levels.peakR,
    peakHoldL: levels.peakHoldL,
    peakHoldR: levels.peakHoldR,
  );
  return prev.copyWith(levels: nextLevels);
}

/// Hydrate every deck and the mixer from one snapshot.
EngineUiSnapshot _applyEngineSnapshot(
  EngineUiSnapshot prev,
  bus.EngineSnapshot snapshot,
) {
  final status = snapshot.status;
  var next = prev.copyWith(
    running: status.running,
    crossfader: status.crossfader,
    masterVolume: status.masterVolume,
    cueVolume: status.cueVolume,
    cueMix: status.cueMix,
    masterCue: status.masterCue,
    masterDeck: status.masterDeck,
  );
  for (final deck in status.decks) {
    // The snapshot's decks are full deck snapshots too, so they go through the
    // same path as a live update; the deck's own id wins over the field.
    next = _applyDeckUpdate(next, _updateFromSnapshot(deck));
  }
  return next;
}

/// Deck snapshot in the shape a live `DeckUpdated` carries.
bus.DeckUpdated _updateFromSnapshot(bus.DeckSnapshot deck) => bus.DeckUpdated(
  deck: deck.id,
  track: deck.track,
  trackId: deck.trackId,
  title: deck.title,
  artist: deck.artist,
  album: deck.album,
  isrc: deck.isrc,
  bpm: deck.bpm,
  key: deck.key,
  playing: deck.playing,
  crossfaderAssign: deck.crossfaderAssign,
  volume: deck.volume,
  speed: deck.speed,
  tempoRange: deck.tempoRange,
  keyLock: deck.keyLock,
  keyShift: deck.keyShift,
  keyboardPage: deck.keyboardPage,
  keyShiftPage: deck.keyShiftPage,
  keyboardRootHotCue: deck.keyboardRootHotCue,
  eq: deck.eq,
  filter: deck.filter,
  gainTrim: deck.gainTrim,
  fxRouted: deck.fxRouted,
  fx: deck.fx,
  headphoneCue: deck.headphoneCue,
  syncMode: deck.syncMode,
  cuePointMs: deck.cuePointMs,
  quantize: deck.quantize,
  activeLoop: deck.activeLoop,
  pendingLoopInMs: deck.pendingLoopInMs,
  slipEnabled: deck.slipEnabled,
  slipShadowPositionMs: deck.slipShadowPositionMs,
  padMode: deck.padMode,
  stemsReady: deck.stemsReady,
  stemsLive: deck.stemsLive,
  stemMute: deck.stemMute,
  stemIsolate: deck.stemIsolate,
  positionMs: deck.positionMs,
  durationMs: deck.durationMs,
  hotCues: deck.hotCues,
  savedLoops: deck.savedLoops,
  loudnessLufs: deck.loudnessLufs,
  autoGainDb: deck.autoGainDb,
  activeSamplerBankId: deck.activeSamplerBankId,
  topJogMode: deck.topJogMode,
  outerJogMode: deck.outerJogMode,
  jogTouching: deck.jogTouching,
  samplerSlots: const [],
);

/// Value comparison for published FX slots; see the `nextFxSlots` note above.
bool _fxSlotsEqual(List<FxSlotState> a, List<FxSlotState> b) {
  if (identical(a, b)) {
    return true;
  }
  if (a.length != b.length) {
    return false;
  }
  for (var index = 0; index < a.length; index++) {
    if (!_fxSlotEqual(a[index], b[index])) {
      return false;
    }
  }
  return true;
}

bool _fxSlotEqual(FxSlotState a, FxSlotState b) =>
    a.kind == b.kind &&
    a.enabled == b.enabled &&
    a.wet == b.wet &&
    listEquals(a.params, b.params);
