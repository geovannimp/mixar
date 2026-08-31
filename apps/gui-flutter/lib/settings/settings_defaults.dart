import 'dart:typed_data';

import 'package:gui_flutter/mixer/key_format.dart';
import 'package:gui_flutter/mixer/waveform/layout.dart';
import 'package:gui_flutter/mixer/waveform/spectral_color.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';

WaveformDisplayMode waveformModeFromSettings(WaveformDisplayModeSetting mode) {
  return switch (mode) {
    WaveformDisplayModeSetting.rgb => WaveformDisplayMode.rgb,
    WaveformDisplayModeSetting.filtered => WaveformDisplayMode.filtered,
  };
}

KeyDisplayMode keyModeFromSettings(KeyDisplayModeSetting mode) {
  return switch (mode) {
    KeyDisplayModeSetting.musical => KeyDisplayMode.musical,
    KeyDisplayModeSetting.camelot => KeyDisplayMode.camelot,
  };
}

const kDefaultBackend = 'cpal';
const kDefaultSampleRate = 48000;
const kDefaultBufferSize = 512;
const kDefaultTargetLufs = -18.0;
const kMinTargetLufs = -24.0;
const kMaxTargetLufs = -9.0;
const kDefaultTempoRange = 0.06;

const kDefaultMasterBus = BusRouteSettings(
  deviceId: 'default',
  leftChannel: 1,
  rightChannel: 2,
  mode: BusChannelMode.stereo,
);

const kDefaultPreviewBus = BusRouteSettings(
  deviceId: 'default',
  leftChannel: 3,
  rightChannel: 4,
  mode: BusChannelMode.stereo,
);

const kLibraryColumnDefs = [
  (id: 'title', label: 'Title', required: true),
  (id: 'artist', label: 'Artist', required: false),
  (id: 'album', label: 'Album', required: false),
  (id: 'genre', label: 'Genre', required: false),
  (id: 'bpm', label: 'BPM', required: false),
  (id: 'key', label: 'Key', required: false),
  (id: 'duration', label: 'Length', required: false),
  (id: 'path', label: 'Path', required: false),
];

const kDefaultLibraryColumns = ['title', 'artist', 'bpm', 'key', 'duration'];

AppSettings defaultAppSettings() {
  return AppSettings(
    backend: kDefaultBackend,
    sampleRate: kDefaultSampleRate,
    bufferSize: kDefaultBufferSize,
    lowLatency: false,
    resamplerQuality: 'medium',
    masterBus: kDefaultMasterBus,
    previewEnabled: false,
    previewBus: kDefaultPreviewBus,
    analysisDuration: AnalysisDurationSetting.precise,
    libraryTableColumns: List<String>.from(kDefaultLibraryColumns),
    volumeNormalizerEnabled: true,
    targetLufs: kDefaultTargetLufs,
    samplerPlayMode: SamplerPlayModeSetting.oneshot,
    samplerStripRoute: SamplerStripRouteSettingFrb.before,
    deckDefaultSamplerBankId: [null, null],
    defaultTopJogMode: JogModeSetting.vinyl,
    defaultOuterJogMode: JogModeSetting.pitchBend,
    defaultTempoRange: kDefaultTempoRange,
    tempoRangeSteps: Float32List.fromList([0.06, 0.10, 0.16, 0.25]),
    defaultKeyLock: false,
    waveformDisplayMode: WaveformDisplayModeSetting.rgb,
    waveformVisibleMs: kWaveformVisibleMs,
    keyDisplayMode: KeyDisplayModeSetting.musical,
    trustedControllerDeviceIds: const [],
    historyEnabled: true,
    historySessionIdleMinutes: 5,
    historyMinPlaySeconds: 5,
    historyMinDeckVolume: 0.05,
  );
}

AppSettings normalizeAppSettings(AppSettings settings) {
  final target = settings.targetLufs.isFinite
      ? settings.targetLufs.clamp(kMinTargetLufs, kMaxTargetLufs)
      : kDefaultTargetLufs;
  final allowed = kLibraryColumnDefs.map((c) => c.id).toSet();
  final columns = settings.libraryTableColumns
      .where(allowed.contains)
      .toList(growable: true);
  if (!columns.contains('title')) {
    columns.insert(0, 'title');
  }
  final banks = List<String?>.from(settings.deckDefaultSamplerBankId);
  while (banks.length < 2) {
    banks.add(null);
  }
  if (banks.length > 2) {
    banks.removeRange(2, banks.length);
  }
  return copyAppSettings(
    settings,
    targetLufs: target,
    libraryTableColumns: columns,
    deckDefaultSamplerBankId: banks,
    masterBus: _normalizeBus(settings.masterBus),
    previewBus: _normalizeBus(settings.previewBus),
    waveformVisibleMs: clampWaveformVisibleMs(settings.waveformVisibleMs),
  );
}

BusRouteSettings _normalizeBus(BusRouteSettings route) {
  return BusRouteSettings(
    deviceId: route.deviceId,
    leftChannel: route.leftChannel,
    rightChannel: route.rightChannel,
    mode: route.mode == BusChannelMode.mono
        ? BusChannelMode.mono
        : BusChannelMode.stereo,
  );
}

AppSettings copyAppSettings(
  AppSettings base, {
  String? backend,
  int? sampleRate,
  int? bufferSize,
  bool? lowLatency,
  String? resamplerQuality,
  BusRouteSettings? masterBus,
  bool? previewEnabled,
  BusRouteSettings? previewBus,
  AnalysisDurationSetting? analysisDuration,
  List<String>? libraryTableColumns,
  bool? volumeNormalizerEnabled,
  double? targetLufs,
  SamplerPlayModeSetting? samplerPlayMode,
  SamplerStripRouteSettingFrb? samplerStripRoute,
  List<String?>? deckDefaultSamplerBankId,
  JogModeSetting? defaultTopJogMode,
  JogModeSetting? defaultOuterJogMode,
  double? defaultTempoRange,
  Float32List? tempoRangeSteps,
  bool? defaultKeyLock,
  WaveformDisplayModeSetting? waveformDisplayMode,
  int? waveformVisibleMs,
  KeyDisplayModeSetting? keyDisplayMode,
  List<String>? trustedControllerDeviceIds,
  bool? historyEnabled,
  int? historySessionIdleMinutes,
  int? historyMinPlaySeconds,
  double? historyMinDeckVolume,
}) {
  return AppSettings(
    backend: backend ?? base.backend,
    sampleRate: sampleRate ?? base.sampleRate,
    bufferSize: bufferSize ?? base.bufferSize,
    lowLatency: lowLatency ?? base.lowLatency,
    resamplerQuality: resamplerQuality ?? base.resamplerQuality,
    masterBus: masterBus ?? base.masterBus,
    previewEnabled: previewEnabled ?? base.previewEnabled,
    previewBus: previewBus ?? base.previewBus,
    analysisDuration: analysisDuration ?? base.analysisDuration,
    libraryTableColumns: libraryTableColumns ?? base.libraryTableColumns,
    volumeNormalizerEnabled:
        volumeNormalizerEnabled ?? base.volumeNormalizerEnabled,
    targetLufs: targetLufs ?? base.targetLufs,
    samplerPlayMode: samplerPlayMode ?? base.samplerPlayMode,
    samplerStripRoute: samplerStripRoute ?? base.samplerStripRoute,
    deckDefaultSamplerBankId:
        deckDefaultSamplerBankId ?? base.deckDefaultSamplerBankId,
    defaultTopJogMode: defaultTopJogMode ?? base.defaultTopJogMode,
    defaultOuterJogMode: defaultOuterJogMode ?? base.defaultOuterJogMode,
    defaultTempoRange: defaultTempoRange ?? base.defaultTempoRange,
    tempoRangeSteps: tempoRangeSteps ?? base.tempoRangeSteps,
    defaultKeyLock: defaultKeyLock ?? base.defaultKeyLock,
    waveformDisplayMode: waveformDisplayMode ?? base.waveformDisplayMode,
    waveformVisibleMs: waveformVisibleMs ?? base.waveformVisibleMs,
    keyDisplayMode: keyDisplayMode ?? base.keyDisplayMode,
    trustedControllerDeviceIds:
        trustedControllerDeviceIds ?? base.trustedControllerDeviceIds,
    historyEnabled: historyEnabled ?? base.historyEnabled,
    historySessionIdleMinutes:
        historySessionIdleMinutes ?? base.historySessionIdleMinutes,
    historyMinPlaySeconds: historyMinPlaySeconds ?? base.historyMinPlaySeconds,
    historyMinDeckVolume: historyMinDeckVolume ?? base.historyMinDeckVolume,
  );
}

/// Value equality for dirty checks. Generated `AppSettings.==` uses collection
/// identity for lists / `Float32List`.
bool appSettingsDirty(AppSettings draft, AppSettings baseline) {
  if (identical(draft, baseline)) {
    return false;
  }
  return draft.backend != baseline.backend ||
      draft.sampleRate != baseline.sampleRate ||
      draft.bufferSize != baseline.bufferSize ||
      draft.lowLatency != baseline.lowLatency ||
      draft.resamplerQuality != baseline.resamplerQuality ||
      draft.masterBus != baseline.masterBus ||
      draft.previewEnabled != baseline.previewEnabled ||
      draft.previewBus != baseline.previewBus ||
      draft.analysisDuration != baseline.analysisDuration ||
      draft.volumeNormalizerEnabled != baseline.volumeNormalizerEnabled ||
      draft.targetLufs != baseline.targetLufs ||
      draft.samplerPlayMode != baseline.samplerPlayMode ||
      draft.samplerStripRoute != baseline.samplerStripRoute ||
      draft.defaultTopJogMode != baseline.defaultTopJogMode ||
      draft.defaultOuterJogMode != baseline.defaultOuterJogMode ||
      draft.defaultTempoRange != baseline.defaultTempoRange ||
      draft.defaultKeyLock != baseline.defaultKeyLock ||
      draft.waveformDisplayMode != baseline.waveformDisplayMode ||
      draft.waveformVisibleMs != baseline.waveformVisibleMs ||
      draft.keyDisplayMode != baseline.keyDisplayMode ||
      draft.historyEnabled != baseline.historyEnabled ||
      draft.historySessionIdleMinutes != baseline.historySessionIdleMinutes ||
      draft.historyMinPlaySeconds != baseline.historyMinPlaySeconds ||
      draft.historyMinDeckVolume != baseline.historyMinDeckVolume ||
      !_sameList(draft.libraryTableColumns, baseline.libraryTableColumns) ||
      !_sameList(
        draft.deckDefaultSamplerBankId,
        baseline.deckDefaultSamplerBankId,
      ) ||
      !_sameList(draft.tempoRangeSteps, baseline.tempoRangeSteps) ||
      !_sameList(
        draft.trustedControllerDeviceIds,
        baseline.trustedControllerDeviceIds,
      );
}

bool _sameList<T>(List<T> a, List<T> b) {
  if (identical(a, b)) {
    return true;
  }
  if (a.length != b.length) {
    return false;
  }
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) {
      return false;
    }
  }
  return true;
}
