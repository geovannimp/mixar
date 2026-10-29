import 'dart:typed_data';

import 'package:gui_flutter/core/models/bus_enums.dart';
import 'package:gui_flutter/src/rust/api/event_mirrors.dart';

/// Bus events built for tests.
///
/// The generated payloads have no defaults (they mirror the Rust structs field
/// for field), so a test that only cares about one or two fields would
/// otherwise repeat forty-odd `null`s. These builders carry the engine's own
/// boot defaults.
DeckUpdated deckUpdated({
  int deck = 0,
  String? trackPath,
  String? trackId,
  String? title,
  String? artist,
  bool playing = false,
  CrossfaderAssign crossfaderAssign = CrossfaderAssign.a,
  double volume = 1,
  double speed = 0.5,
  double tempoRange = 0.06,
  bool keyLock = false,
  double keyShift = 0,
  int keyboardPage = 2,
  int keyShiftPage = 2,
  int keyboardRootHotCue = 0,
  DeckEq eq = const DeckEq(low: 0.5, mid: 0.5, high: 0.5),
  double filter = 0.5,
  double gainTrim = 0.5,
  bool fxRouted = false,
  List<FxSlotState>? fx,
  bool headphoneCue = false,
  SyncMode syncMode = SyncMode.off,
  int? cuePointMs,
  bool quantize = true,
  LoopRegion? activeLoop,
  int? pendingLoopInMs,
  bool slipEnabled = false,
  int? slipShadowPositionMs,
  PadMode padMode = PadMode.hotCue,
  bool stemsReady = false,
  bool stemsLive = false,
  List<bool> stemMute = const [false, false, false, false],
  int? stemIsolate,
  int? positionMs,
  int? durationMs,
  List<DeckHotCue> hotCues = const [],
  List<DeckSavedLoop> savedLoops = const [],
  double? loudnessLufs,
  double autoGainDb = 0,
  String? activeSamplerBankId,
  JogMode topJogMode = JogMode.vinyl,
  JogMode outerJogMode = JogMode.vinyl,
  bool jogTouching = false,
  List<SamplerSlotInfo> samplerSlots = const [],
}) => DeckUpdated(
  deck: deck,
  track: trackPath,
  trackId: trackId,
  title: title,
  artist: artist,
  playing: playing,
  crossfaderAssign: crossfaderAssign,
  volume: volume,
  speed: speed,
  tempoRange: tempoRange,
  keyLock: keyLock,
  keyShift: keyShift,
  keyboardPage: keyboardPage,
  keyShiftPage: keyShiftPage,
  keyboardRootHotCue: keyboardRootHotCue,
  eq: eq,
  filter: filter,
  gainTrim: gainTrim,
  fxRouted: fxRouted,
  fx: fx ?? defaultFxSlots,
  headphoneCue: headphoneCue,
  syncMode: syncMode,
  cuePointMs: cuePointMs,
  quantize: quantize,
  activeLoop: activeLoop,
  pendingLoopInMs: pendingLoopInMs,
  slipEnabled: slipEnabled,
  slipShadowPositionMs: slipShadowPositionMs,
  padMode: padMode,
  stemsReady: stemsReady,
  stemsLive: stemsLive,
  stemMute: stemMute,
  stemIsolate: stemIsolate,
  positionMs: positionMs,
  durationMs: durationMs,
  hotCues: hotCues,
  savedLoops: savedLoops,
  loudnessLufs: loudnessLufs,
  autoGainDb: autoGainDb,
  activeSamplerBankId: activeSamplerBankId,
  topJogMode: topJogMode,
  outerJogMode: outerJogMode,
  jogTouching: jogTouching,
  samplerSlots: samplerSlots,
);

/// The shared FX unit's boot state: one armed-off Echo slot.
final defaultFxSlots = <FxSlotState>[
  FxSlotState(
    kind: FxKind.echo,
    enabled: false,
    wet: 0.5,
    params: Float32List.fromList(const [0.5, 0.5, 0.0]),
  ),
];

/// One `DeckUpdated` bus event for [deck].
Event deckUpdatedEvent({int deck = 0, DeckUpdated? update}) =>
    Event.deckUpdated(update ?? deckUpdated(deck: deck));

/// One `DeckPosition` bus event.
Event deckPositionEvent({
  int deck = 0,
  required int positionMs,
  int? slipShadowPositionMs,
}) => Event.deckPosition(
  DeckPosition(
    deck: deck,
    positionMs: positionMs,
    slipShadowPositionMs: slipShadowPositionMs,
  ),
);

/// One `DeckLevels` bus event; [deck] `null` is the master bus.
Event deckLevelsEvent({
  int? deck,
  double peakL = 0,
  double peakR = 0,
  double peakHoldL = 0,
  double peakHoldR = 0,
}) => Event.deckLevels(
  DeckLevels(
    deck: deck,
    peakL: peakL,
    peakR: peakR,
    peakHoldL: peakHoldL,
    peakHoldR: peakHoldR,
  ),
);

/// One `EngineSnapshot` bus event hydrating [decks].
Event engineSnapshotEvent({
  bool running = true,
  double crossfader = 0.5,
  double masterVolume = 1,
  double cueVolume = 1,
  double cueMix = 0.5,
  bool masterCue = false,
  int masterDeck = 0,
  List<DeckSnapshot> decks = const [],
}) => Event.engineSnapshot(
  EngineSnapshot(
    status: EngineStatus(
      running: running,
      sampleRate: 48000,
      crossfader: crossfader,
      masterVolume: masterVolume,
      cueVolume: cueVolume,
      cueMix: cueMix,
      masterCue: masterCue,
      masterDeck: masterDeck,
      decks: decks,
    ),
  ),
);

/// A deck snapshot shaped like the ones an engine status carries.
DeckSnapshot deckSnapshot({
  int deck = 0,
  String? trackId,
  String? title,
  String? artist,
  bool playing = false,
  double volume = 1,
  int? durationMs,
  int? positionMs,
  bool headphoneCue = false,
  PadMode padMode = PadMode.hotCue,
}) => DeckSnapshot(
  id: deck,
  track: null,
  trackId: trackId,
  title: title,
  artist: artist,
  album: null,
  isrc: null,
  bpm: null,
  key: null,
  playing: playing,
  crossfaderAssign: CrossfaderAssign.a,
  volume: volume,
  speed: 0.5,
  tempoRange: 0.06,
  keyLock: false,
  keyShift: 0,
  keyboardPage: 2,
  keyShiftPage: 2,
  keyboardRootHotCue: 0,
  eq: const DeckEq(low: 0.5, mid: 0.5, high: 0.5),
  filter: 0.5,
  gainTrim: 0.5,
  fxRouted: false,
  fx: defaultFxSlots,
  headphoneCue: headphoneCue,
  syncMode: SyncMode.off,
  cuePointMs: null,
  quantize: true,
  activeLoop: null,
  pendingLoopInMs: null,
  slipEnabled: false,
  slipShadowPositionMs: null,
  padMode: padMode,
  stemsReady: false,
  stemsLive: false,
  stemMute: const [false, false, false, false],
  stemIsolate: null,
  positionMs: positionMs,
  durationMs: durationMs,
  hotCues: const [],
  savedLoops: const [],
  loudnessLufs: null,
  autoGainDb: 0,
  activeSamplerBankId: null,
  topJogMode: JogMode.vinyl,
  outerJogMode: JogMode.vinyl,
  jogTouching: false,
);
