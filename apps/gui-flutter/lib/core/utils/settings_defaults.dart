import 'dart:typed_data';

import 'package:gui_flutter/core/utils/desktop.dart';
import 'package:gui_flutter/core/utils/key_format.dart';
import 'package:gui_flutter/core/utils/tempo_format.dart';
import 'package:gui_flutter/core/utils/waveform_spectral_color.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';

/// Built-in offline separation models (must match analyzer-stems).
const kStemModels = ['htdemucs_mixxx_v1', 'scnet_int8_v1'];
const kDefaultStemModel = 'htdemucs_mixxx_v1';

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

KeyColorMode keyColorModeFromSettings(KeyColorModeSetting mode) {
  return switch (mode) {
    KeyColorModeSetting.off => KeyColorMode.off,
    KeyColorModeSetting.absolute => KeyColorMode.absolute,
    KeyColorModeSetting.harmonic => KeyColorMode.harmonic,
  };
}

/// Library list rows: one dense line, or the two-line default.
///
/// `comfortable` is sized for a title plus one run of metadata pills, so a long
/// artist name ellipsizes on the title line instead of overflowing the row.
enum LibraryRowDensity {
  compact(height: 36, artSize: 28),
  comfortable(height: 60, artSize: 40);

  new({required this.height, required this.artSize});

  final double height;
  final double artSize;

  bool get isCompact => this == LibraryRowDensity.compact;

  LibraryRowDensity get other => this == LibraryRowDensity.compact
      ? LibraryRowDensity.comfortable
      : LibraryRowDensity.compact;
}

/// Resolve [SelectStyleSetting.auto] from the host OS via [isDesktopWindow].
///
/// Never returns [SelectStyleSetting.auto].
SelectStyleSetting effectiveSelectStyle(SelectStyleSetting setting) {
  return switch (setting) {
    SelectStyleSetting.auto =>
      isDesktopWindow ? SelectStyleSetting.desktop : SelectStyleSetting.mobile,
    SelectStyleSetting.desktop => SelectStyleSetting.desktop,
    SelectStyleSetting.mobile => SelectStyleSetting.mobile,
  };
}

LibraryRowDensity libraryRowDensityFromSettings(
  LibraryRowDensitySetting setting,
) => switch (setting) {
  LibraryRowDensitySetting.compact => LibraryRowDensity.compact,
  LibraryRowDensitySetting.comfortable => LibraryRowDensity.comfortable,
};

const kDefaultSampleRate = 48000;
const kDefaultBufferSize = 512;
const kDefaultTargetLufs = -18.0;
const kMinTargetLufs = -24.0;
const kMaxTargetLufs = -9.0;
const kDefaultVinylBrakeStopSecs = 0.5;
const kMinVinylBrakeStopSecs = 0.05;
const kMaxVinylBrakeStopSecs = 3.0;
const kVinylBrakeStopSecsStep = 0.05;

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

/// Row fields a fresh install shows — today's layout: album/genre/length pills
/// under the title, rating/colour/BPM/key trailing. Must match the Rust
/// `default_library_row_fields`.
const kDefaultLibraryRowFields = <LibraryRowFieldSetting>[
  LibraryRowFieldSetting.album,
  LibraryRowFieldSetting.genre,
  LibraryRowFieldSetting.duration,
  LibraryRowFieldSetting.rating,
  LibraryRowFieldSetting.color,
  LibraryRowFieldSetting.bpm,
  LibraryRowFieldSetting.key,
];

AppSettings defaultAppSettings() {
  return AppSettings(
    sampleRate: kDefaultSampleRate,
    bufferSize: kDefaultBufferSize,
    lowLatency: false,
    resamplerQuality: 'medium',
    masterBus: kDefaultMasterBus,
    previewEnabled: false,
    previewBus: kDefaultPreviewBus,
    analysisDuration: AnalysisDurationSetting.precise,
    libraryRowDensity: LibraryRowDensitySetting.comfortable,
    libraryRowFields: kDefaultLibraryRowFields,
    tagWriteBack: false,
    volumeNormalizerEnabled: true,
    targetLufs: kDefaultTargetLufs,
    samplerPlayMode: SamplerPlayModeSetting.oneshot,
    samplerStripRoute: SamplerStripRouteSettingFrb.before,
    numDecks: 2,
    deckDefaultSamplerBankId: [null, null],
    defaultTopJogMode: JogModeSetting.vinyl,
    defaultOuterJogMode: JogModeSetting.pitchBend,
    defaultTempoRange: kDefaultTempoRange,
    tempoRangeSteps: Float32List.fromList([0.06, 0.10, 0.16, 0.25]),
    defaultKeyLock: false,
    vinylBrakePause: false,
    vinylBrakeStopSecs: kDefaultVinylBrakeStopSecs,
    waveformDisplayMode: WaveformDisplayModeSetting.rgb,
    keyDisplayMode: KeyDisplayModeSetting.musical,
    keyColorMode: KeyColorModeSetting.off,
    trustedControllerDeviceIds: const [],
    historyEnabled: true,
    historySessionIdleMinutes: 5,
    historyMinPlaySeconds: 5,
    historyMinDeckVolume: 0.05,
    showTooltips: true,
    showMixerMeters: true,
    mixerMetersMono: true,
    selectStyle: SelectStyleSetting.auto,
    dimPlayedTracks: true,
    stemsFormat: 'opus',
    stemsModel: kDefaultStemModel,
    realtimeStems: false,
    uiLanguage: UiLanguageSetting.system,
    libraryDrawerEnabled: false,
    libraryDrawerSide: LibraryDrawerSideSetting.bottom,
    analyticsEnabled: false,
  );
}

AppSettings normalizeAppSettings(AppSettings settings) {
  final target = settings.targetLufs.isFinite
      ? settings.targetLufs.clamp(kMinTargetLufs, kMaxTargetLufs)
      : kDefaultTargetLufs;
  final numDecks = settings.numDecks == 4 ? 4 : 2;
  final banks = List<String?>.from(settings.deckDefaultSamplerBankId);
  while (banks.length < numDecks) {
    banks.add(null);
  }
  if (banks.length > numDecks) {
    banks.removeRange(numDecks, banks.length);
  }
  return copyAppSettings(
    settings,
    targetLufs: target,
    numDecks: numDecks,
    deckDefaultSamplerBankId: banks,
    masterBus: _normalizeBus(settings.masterBus),
    previewBus: _normalizeBus(settings.previewBus),
    stemsModel: kStemModels.contains(settings.stemsModel)
        ? settings.stemsModel
        : kDefaultStemModel,
    libraryRowFields: _dedupeFields(settings.libraryRowFields),
  );
}

/// Keep the first occurrence of each field: the order is the row's priority,
/// and a duplicate would paint the same pill twice.
List<LibraryRowFieldSetting> _dedupeFields(
  List<LibraryRowFieldSetting> fields,
) {
  final seen = <LibraryRowFieldSetting>[];
  for (final field in fields) {
    if (!seen.contains(field)) {
      seen.add(field);
    }
  }
  return seen;
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
  int? sampleRate,
  int? bufferSize,
  bool? lowLatency,
  String? resamplerQuality,
  BusRouteSettings? masterBus,
  bool? previewEnabled,
  BusRouteSettings? previewBus,
  AnalysisDurationSetting? analysisDuration,
  LibraryRowDensitySetting? libraryRowDensity,
  List<LibraryRowFieldSetting>? libraryRowFields,
  bool? tagWriteBack,
  bool? volumeNormalizerEnabled,
  double? targetLufs,
  SamplerPlayModeSetting? samplerPlayMode,
  SamplerStripRouteSettingFrb? samplerStripRoute,
  int? numDecks,
  List<String?>? deckDefaultSamplerBankId,
  JogModeSetting? defaultTopJogMode,
  JogModeSetting? defaultOuterJogMode,
  double? defaultTempoRange,
  Float32List? tempoRangeSteps,
  bool? defaultKeyLock,
  bool? vinylBrakePause,
  double? vinylBrakeStopSecs,
  WaveformDisplayModeSetting? waveformDisplayMode,
  KeyDisplayModeSetting? keyDisplayMode,
  KeyColorModeSetting? keyColorMode,
  List<String>? trustedControllerDeviceIds,
  bool? historyEnabled,
  int? historySessionIdleMinutes,
  int? historyMinPlaySeconds,
  double? historyMinDeckVolume,
  bool? showTooltips,
  bool? showMixerMeters,
  bool? mixerMetersMono,
  SelectStyleSetting? selectStyle,
  bool? dimPlayedTracks,
  String? stemsFormat,
  String? stemsModel,
  bool? realtimeStems,
  UiLanguageSetting? uiLanguage,
  bool? libraryDrawerEnabled,
  LibraryDrawerSideSetting? libraryDrawerSide,
  bool? analyticsEnabled,
}) {
  return AppSettings(
    numDecks: numDecks ?? base.numDecks,
    sampleRate: sampleRate ?? base.sampleRate,
    bufferSize: bufferSize ?? base.bufferSize,
    lowLatency: lowLatency ?? base.lowLatency,
    resamplerQuality: resamplerQuality ?? base.resamplerQuality,
    masterBus: masterBus ?? base.masterBus,
    previewEnabled: previewEnabled ?? base.previewEnabled,
    previewBus: previewBus ?? base.previewBus,
    analysisDuration: analysisDuration ?? base.analysisDuration,
    libraryRowDensity: libraryRowDensity ?? base.libraryRowDensity,
    libraryRowFields: libraryRowFields ?? base.libraryRowFields,
    tagWriteBack: tagWriteBack ?? base.tagWriteBack,
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
    vinylBrakePause: vinylBrakePause ?? base.vinylBrakePause,
    vinylBrakeStopSecs: vinylBrakeStopSecs ?? base.vinylBrakeStopSecs,
    waveformDisplayMode: waveformDisplayMode ?? base.waveformDisplayMode,
    keyDisplayMode: keyDisplayMode ?? base.keyDisplayMode,
    keyColorMode: keyColorMode ?? base.keyColorMode,
    trustedControllerDeviceIds:
        trustedControllerDeviceIds ?? base.trustedControllerDeviceIds,
    historyEnabled: historyEnabled ?? base.historyEnabled,
    historySessionIdleMinutes:
        historySessionIdleMinutes ?? base.historySessionIdleMinutes,
    historyMinPlaySeconds: historyMinPlaySeconds ?? base.historyMinPlaySeconds,
    historyMinDeckVolume: historyMinDeckVolume ?? base.historyMinDeckVolume,
    showTooltips: showTooltips ?? base.showTooltips,
    showMixerMeters: showMixerMeters ?? base.showMixerMeters,
    mixerMetersMono: mixerMetersMono ?? base.mixerMetersMono,
    selectStyle: selectStyle ?? base.selectStyle,
    dimPlayedTracks: dimPlayedTracks ?? base.dimPlayedTracks,
    stemsFormat: stemsFormat ?? base.stemsFormat,
    stemsModel: stemsModel ?? base.stemsModel,
    realtimeStems: realtimeStems ?? base.realtimeStems,
    uiLanguage: uiLanguage ?? base.uiLanguage,
    libraryDrawerEnabled: libraryDrawerEnabled ?? base.libraryDrawerEnabled,
    libraryDrawerSide: libraryDrawerSide ?? base.libraryDrawerSide,
    analyticsEnabled: analyticsEnabled ?? base.analyticsEnabled,
  );
}

/// Value equality for dirty checks. Generated `AppSettings.==` uses collection
/// identity for lists / `Float32List`.
bool appSettingsDirty(AppSettings draft, AppSettings baseline) {
  if (identical(draft, baseline)) {
    return false;
  }
  return draft.numDecks != baseline.numDecks ||
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
      draft.vinylBrakePause != baseline.vinylBrakePause ||
      !_sameDuration(draft.vinylBrakeStopSecs, baseline.vinylBrakeStopSecs) ||
      draft.waveformDisplayMode != baseline.waveformDisplayMode ||
      draft.keyDisplayMode != baseline.keyDisplayMode ||
      draft.keyColorMode != baseline.keyColorMode ||
      draft.historyEnabled != baseline.historyEnabled ||
      draft.historySessionIdleMinutes != baseline.historySessionIdleMinutes ||
      draft.historyMinPlaySeconds != baseline.historyMinPlaySeconds ||
      draft.historyMinDeckVolume != baseline.historyMinDeckVolume ||
      draft.showTooltips != baseline.showTooltips ||
      draft.showMixerMeters != baseline.showMixerMeters ||
      draft.mixerMetersMono != baseline.mixerMetersMono ||
      draft.selectStyle != baseline.selectStyle ||
      draft.dimPlayedTracks != baseline.dimPlayedTracks ||
      draft.stemsFormat != baseline.stemsFormat ||
      draft.stemsModel != baseline.stemsModel ||
      draft.realtimeStems != baseline.realtimeStems ||
      draft.uiLanguage != baseline.uiLanguage ||
      draft.libraryDrawerEnabled != baseline.libraryDrawerEnabled ||
      draft.libraryDrawerSide != baseline.libraryDrawerSide ||
      draft.analyticsEnabled != baseline.analyticsEnabled ||
      draft.libraryRowDensity != baseline.libraryRowDensity ||
      !_sameList(draft.libraryRowFields, baseline.libraryRowFields) ||
      draft.tagWriteBack != baseline.tagWriteBack ||
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

/// Stop secs crosses the Rust f32 boundary, so the host baseline and a stepper
/// draft can describe the same displayed value with different doubles. Compare
/// on a tolerance far below the 0.05 step so a genuine change still reads dirty.
bool _sameDuration(double a, double b) => (a - b).abs() <= 1e-6;

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
