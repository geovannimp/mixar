/// Matches `codec` `SUPPORTED_AUDIO_EXTENSIONS`.
const supportedAudioExtensions = [
  'mp3',
  'flac',
  'wav',
  'aiff',
  'aif',
  'ogg',
  'm4a',
  'aac',
  'opus',
  'wma',
  'alac',
];

enum TrackDragSource { library, filesystem }

enum TrackLoadKind { library, path }

class TrackDragPayload {
  const new({
    required this.source,
    required this.path,
    required this.title,
    this.trackId,
  });

  final TrackDragSource source;
  final String? trackId;
  final String path;
  final String title;

  @override
  int get hashCode =>
      source.hashCode ^ trackId.hashCode ^ path.hashCode ^ title.hashCode;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is TrackDragPayload &&
          source == other.source &&
          trackId == other.trackId &&
          path == other.path &&
          title == other.title;
}

TrackLoadKind trackLoadKind(TrackDragPayload payload) {
  if (payload.source == TrackDragSource.library &&
      payload.trackId != null &&
      payload.trackId!.isNotEmpty) {
    return TrackLoadKind.library;
  }
  return TrackLoadKind.path;
}

String fileNameFromPath(String path) {
  final base = path.replaceAll(r'\', '/').split('/').last;
  return base.isEmpty ? path : base;
}

/// File name without the last extension segment.
String fileStemFromPath(String path) {
  final base = fileNameFromPath(path);
  final dot = base.lastIndexOf('.');
  if (dot > 0) {
    return base.substring(0, dot);
  }
  return base;
}

/// Metadata title when present; otherwise the file name so a loaded deck
/// never looks empty.
String trackDisplayTitle({required String title, required String path}) {
  final trimmed = title.trim();
  if (trimmed.isNotEmpty) {
    return trimmed;
  }
  return fileNameFromPath(path);
}

TrackDragPayload payloadFromOsPath(String path) {
  return TrackDragPayload(
    source: TrackDragSource.filesystem,
    path: path,
    title: fileNameFromPath(path),
  );
}

bool isSupportedAudioPath(String path) {
  final base = fileNameFromPath(path);
  final dot = base.lastIndexOf('.');
  if (dot < 0 || dot == base.length - 1) {
    return false;
  }
  final ext = base.substring(dot + 1).toLowerCase();
  return supportedAudioExtensions.contains(ext);
}

List<String> filterAudioFilePaths(Iterable<String> paths) => [
  for (final path in paths)
    if (isSupportedAudioPath(path)) path,
];

Future<void> applyTrackDrop({
  required int deckId,
  required TrackDragPayload payload,
  required Future<void> Function(int deckId, String trackId) loadLibraryTrack,
  required Future<void> Function(int deckId, String path) loadPath,
}) {
  if (trackLoadKind(payload) == TrackLoadKind.library) {
    return loadLibraryTrack(deckId, payload.trackId!);
  }
  return loadPath(deckId, payload.path);
}
