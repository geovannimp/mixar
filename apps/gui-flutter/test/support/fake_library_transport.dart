// Test doubles for LibraryTransport (FRB opaque). Hand-maintained stubs.
import 'dart:async';

import 'package:gui_flutter/src/rust/api/library.dart';

typedef GetTrackHandler = Future<LibraryTrackSummary?> Function(String trackId);
typedef ListCollectionEntriesHandler =
    Future<List<LibraryTrackSummary>> Function(String collectionId);

/// One recorded `LibraryTransport.updateTrackMetadata` call.
///
/// Mirrors the transport signature, so a parameter change to the method (or
/// to this record) is a compile error at the call sites instead of drift.
typedef UpdateTrackMetadataHandler = Future<void> Function(
  UpdateTrackMetadataCall call,
);

typedef UpdateTrackMetadataCall = ({
  String trackId,
  String? title,
  String? artist,
  String? album,
  String? genre,
  double? bpm,
  String? key,
  String? isrc,
});

/// One captured `exportCdjUsb` request.
typedef CdjExportRequest = ({
  String destination,
  List<String> playlists,
  List<String> folders,
  bool refreshFiles,
});

typedef ExportCdjUsbHandler = Future<CdjExportReport> Function(
  CdjExportRequest request,
);

/// Minimal [LibraryTransport] fake: override getTrack / list / events as needed.
class FakeLibraryTransport implements LibraryTransport {
  FakeLibraryTransport({
    this.getTrackHandler,
    this.listCollectionEntriesHandler,
    this.exportCdjUsbHandler,
    this.entries = const [],
    this.collections = const [],
    StreamController<LibraryEvt>? events,
  }) : _events = events ?? StreamController<LibraryEvt>.broadcast();

  GetTrackHandler? getTrackHandler;
  ListCollectionEntriesHandler? listCollectionEntriesHandler;

  /// Optional hook so a test can make a save fail (or inspect the one value
  /// being written). Called after the call is recorded; it may throw.
  UpdateTrackMetadataHandler? updateTrackMetadataHandler;
  ExportCdjUsbHandler? exportCdjUsbHandler;
  List<LibraryTrackSummary> entries;
  List<LibraryCollectionSummary> collections;
  final StreamController<LibraryEvt> _events;
  final List<String> getTrackCalls = [];
  final List<String> listCollectionEntriesCalls = [];
  final List<UpdateTrackMetadataCall> updateTrackMetadataCalls = [];
  final List<CdjExportRequest> exportCdjUsbCalls = [];
  int outstandingGetTracks = 0;
  int maxOutstandingGetTracks = 0;
  var _disposed = false;

  StreamController<LibraryEvt> get events => _events;

  @override
  void dispose() {
    if (_disposed) {
      return;
    }
    _disposed = true;
    if (!_events.isClosed) {
      unawaited(_events.close());
    }
  }

  @override
  bool get isDisposed => _disposed || _events.isClosed;

  @override
  Future<LibraryTrackSummary?> getTrack({required String trackId}) {
    getTrackCalls.add(trackId);
    outstandingGetTracks++;
    if (outstandingGetTracks > maxOutstandingGetTracks) {
      maxOutstandingGetTracks = outstandingGetTracks;
    }
    // Avoid `async` + `return future` + `finally`: Dart runs that finally before
    // the returned future completes, which under-counts concurrency.
    final handler = getTrackHandler;
    try {
      final result = handler != null
          ? handler(trackId)
          : Future<LibraryTrackSummary?>.value(
              LibraryTrackSummary(
                id: trackId,
                displayName: trackId,
                path: '/$trackId.flac',
              ),
            );
      return result.whenComplete(() {
        outstandingGetTracks--;
      });
    } catch (_) {
      outstandingGetTracks--;
      rethrow;
    }
  }

  @override
  Future<List<LibraryTrackSummary>> listCollectionEntries({
    required String collectionId,
  }) async {
    listCollectionEntriesCalls.add(collectionId);
    final handler = listCollectionEntriesHandler;
    if (handler != null) {
      return handler(collectionId);
    }
    return entries;
  }

  @override
  Future<List<LibraryCollectionSummary>> listCollections() async => collections;

  @override
  Stream<LibraryEvt> subscribeEvents() => _events.stream;

  @override
  Future<AddFolderCollectionResult> addFolderCollection({
    required String folderPath,
    required bool scanFolderTree,
    String? name,
  }) async => throw UnimplementedError('addFolderCollection');

  @override
  Future<LibraryCollectionSummary> addPlaylistCollection({
    required String name,
    required bool sortable,
  }) async => throw UnimplementedError('addPlaylistCollection');

  @override
  Future<AnalyzeCollectionResult> analyzeCollection({
    required String collectionId,
    required bool force,
    required bool generateStems,
  }) async => throw UnimplementedError('analyzeCollection');

  @override
  Future<void> analyzeTrack({
    required String trackId,
    required bool force,
  }) async {}

  @override
  Future<void> applyHistorySettings({
    required bool enabled,
    required int sessionIdleMinutes,
    required int minPlaySeconds,
    required double minDeckVolume,
  }) async {}

  @override
  Future<void> applyLibrarySettings({
    required LibraryAnalysisDurationSetting analysisDuration,
    required String stemsFormat,
    required bool tagWriteBack,
  }) async {}

  @override
  Future<LibraryBusHandle> buses() async => throw UnimplementedError('buses');

  @override
  Future<void> clearModelCache() async {}

  @override
  Future<void> clearStemCache() async {}

  @override
  Future<void> clearWaveformCache() async {}

  @override
  Future<void> deleteHistorySession({required String sessionId}) async {}

  @override
  Future<void> deleteLoop({required String trackId, required int slot}) async {}

  @override
  Future<CdjExportReport> exportCdjUsb({
    required String destination,
    required List<String> playlists,
    required List<String> folders,
    required bool refreshFiles,
  }) {
    final request = (
      destination: destination,
      playlists: playlists,
      folders: folders,
      refreshFiles: refreshFiles,
    );
    exportCdjUsbCalls.add(request);
    final handler = exportCdjUsbHandler;
    if (handler != null) {
      return handler(request);
    }
    throw UnimplementedError('exportCdjUsb');
  }

  @override
  Future<void> exportHistorySession({
    required String sessionId,
    required HistoryExportFormatSetting format,
    required String destPath,
  }) async {}

  @override
  Future<void> generateStems({required String trackId}) async {}

  @override
  Future<BeatGridData?> getBeatGrid({required String trackId}) async => null;

  @override
  Future<WaveformPeaks?> getWaveformOverview({required String trackId}) async =>
      null;

  @override
  Future<WaveformPeaks> getWaveformWindow({
    required String trackId,
    required int startMs,
    required int endMs,
    required int buckets,
  }) async => throw UnimplementedError('getWaveformWindow');

  @override
  Future<bool> historyCanResume() async => false;

  @override
  Future<void> historyDeclineRestore() async {}

  @override
  Future<String> historyDir() async => '';

  @override
  Future<void> historyNewSession() async {}

  @override
  Future<HistoryRestorePromptInfo?> historyRestorePrompt() async => null;

  @override
  Future<void> historyRestoreSession({required String sessionId}) async {}

  @override
  Future<void> historyResumeSession() async {}

  @override
  Future<List<HistoryEntryInfo>> historySessionEntries({
    required String sessionId,
  }) async => const [];

  @override
  Future<MixxxImportReport> importMixxxLibrary({
    required String dbPath,
  }) async => throw UnimplementedError('importMixxxLibrary');

  @override
  Future<List<HistorySessionSummary>> listHistorySessions() async => const [];

  @override
  Future<List<SamplerBankInfo>> listSamplerBanks() async => const [];

  @override
  Future<void> refreshTrack({required String trackId}) async {}

  @override
  Future<void> renameHistorySession({
    required String sessionId,
    required String title,
  }) async {}

  @override
  Future<List<ResolvedLibraryTrack>> resolveTracksForPaths({
    required List<String> paths,
  }) async => const [];

  @override
  Future<void> revealHistoryFolder() async {}

  @override
  Future<void> saveBeatGrid({
    required String trackId,
    required double bpm,
    required double firstBeatSecs,
  }) async {}

  @override
  Future<LibraryCollectionSummary> saveHistoryAsPlaylist({
    required String sessionId,
    required String name,
    required bool sortable,
  }) async => throw UnimplementedError('saveHistoryAsPlaylist');

  @override
  Future<void> saveLoop({
    required String trackId,
    required int slot,
    required int inMs,
    required int outMs,
  }) async {}

  @override
  Future<StorageUsage> storageUsage() async =>
      throw UnimplementedError('storageUsage');

  @override
  Future<int> syncStemCache() async => 0;

  @override
  Future<void> updateTrackMetadata({
    required String trackId,
    String? title,
    String? artist,
    String? album,
    String? genre,
    double? bpm,
    String? key,
    String? isrc,
  }) async {
    final call = (
      trackId: trackId,
      title: title,
      artist: artist,
      album: album,
      genre: genre,
      bpm: bpm,
      key: key,
      isrc: isrc,
    );
    updateTrackMetadataCalls.add(call);
    final handler = updateTrackMetadataHandler;
    if (handler != null) {
      await handler(call);
    }
  }
}
