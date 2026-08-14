import 'dart:convert';

import 'package:gui_flutter/src/rust/api/library.dart';

/// Matches `library-core` `SUPPORTED_AUDIO_EXTENSIONS`.
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
  const TrackDragPayload({
    required this.source,
    required this.path,
    required this.title,
    this.trackId,
  });

  final TrackDragSource source;
  final String? trackId;
  final String path;
  final String title;

  Map<String, Object?> toLocalData() => {
    'type': 'track',
    'source': source.name,
    'trackId': trackId,
    'path': path,
    'title': title,
  };

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
  final base = path.replaceAll('\\', '/').split('/').last;
  return base.isEmpty ? path : base;
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

TrackDragPayload payloadFromLibraryTrack(LibraryTrackSummary track) {
  final inLibrary = track.id != track.path;
  final title = (track.title?.isNotEmpty ?? false)
      ? track.title!
      : track.displayName;
  return TrackDragPayload(
    source: inLibrary ? TrackDragSource.library : TrackDragSource.filesystem,
    trackId: inLibrary ? track.id : null,
    path: track.path,
    title: trackDisplayTitle(title: title, path: track.path),
  );
}

TrackDragPayload? parseTrackDragLocalData(Object? data) {
  if (data is! Map) {
    return null;
  }
  if (data['type'] != 'track') {
    return null;
  }
  final sourceName = data['source'];
  final path = data['path'];
  final title = data['title'];
  if (sourceName is! String || path is! String || title is! String) {
    return null;
  }
  final source = switch (sourceName) {
    'library' => TrackDragSource.library,
    'filesystem' => TrackDragSource.filesystem,
    _ => null,
  };
  if (source == null) {
    return null;
  }
  final trackId = data['trackId'];
  return TrackDragPayload(
    source: source,
    trackId: trackId is String && trackId.isNotEmpty ? trackId : null,
    path: path,
    title: trackDisplayTitle(title: title, path: path),
  );
}

/// GTK needs a platform format on the drag item; `localData` alone yields an
/// empty target list and the drop session never reaches Flutter.
String encodeTrackDragPlainText(TrackDragPayload payload) =>
    jsonEncode(payload.toLocalData());

TrackDragPayload? parseTrackDragPlainText(String? text) {
  if (text == null || text.isEmpty) {
    return null;
  }
  try {
    final payload = parseTrackDragLocalData(jsonDecode(text));
    if (payload == null) {
      return null;
    }
    if (payload.source == TrackDragSource.filesystem &&
        !isSupportedAudioPath(payload.path)) {
      return null;
    }
    return payload;
  } on FormatException {
    return null;
  }
}

/// Prefer copy; Linux GTK often advertises move instead.
String preferredTrackDropOperation(Iterable<String> allowed) {
  final set = allowed.toSet();
  if (set.contains('copy')) {
    return 'copy';
  }
  if (set.contains('move')) {
    return 'move';
  }
  if (set.contains('link')) {
    return 'link';
  }
  return 'copy';
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

String pathFromDroppedUri(Uri uri) {
  if (uri.scheme == 'file' || uri.scheme.isEmpty) {
    return uri.toFilePath();
  }
  return uri.toFilePath();
}

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
