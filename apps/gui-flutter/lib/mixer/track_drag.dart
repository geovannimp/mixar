import 'package:gui_flutter/src/rust/api/engine.dart';
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

TrackDragPayload payloadFromOsPath(String path) {
  final slash = path.replaceAll('\\', '/').lastIndexOf('/');
  final title = slash >= 0 ? path.substring(slash + 1) : path;
  return TrackDragPayload(
    source: TrackDragSource.filesystem,
    path: path,
    title: title.isEmpty ? path : title,
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
    title: title,
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
    title: title,
  );
}

bool isSupportedAudioPath(String path) {
  final base = path.replaceAll('\\', '/').split('/').last;
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

class EngineUiSnapshot {
  const EngineUiSnapshot({required this.running, required this.titles});

  static const empty = EngineUiSnapshot(running: false, titles: {});

  final bool running;
  final Map<int, String> titles;

  String? titleFor(int deckId) => titles[deckId];

  EngineUiSnapshot copyWith({bool? running, Map<int, String>? titles}) =>
      EngineUiSnapshot(
        running: running ?? this.running,
        titles: titles ?? this.titles,
      );
}

EngineUiSnapshot applyEngineEvt(EngineUiSnapshot prev, EngineEvt evt) {
  switch (evt.kind) {
    case EngineEvtKind.status:
      return prev.copyWith(running: evt.running ?? prev.running);
    case EngineEvtKind.updated:
      final id = evt.deckId;
      if (id == null) {
        return prev;
      }
      final next = Map<int, String>.from(prev.titles);
      final title = evt.track;
      if (title == null || title.isEmpty) {
        next.remove(id);
      } else {
        next[id] = title;
      }
      return prev.copyWith(titles: next);
    case EngineEvtKind.position:
    case EngineEvtKind.levels:
    case EngineEvtKind.error:
    case EngineEvtKind.notice:
      return prev;
  }
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
