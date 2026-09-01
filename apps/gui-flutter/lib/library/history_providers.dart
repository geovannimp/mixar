import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/library/history_refresh.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/mixer/fader_slider.dart';
import 'package:gui_flutter/mixer/track_drag.dart';
import 'package:gui_flutter/settings/settings_providers.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';

export 'package:gui_flutter/library/history_refresh.dart'
    show historyRefreshTickProvider, HistoryRefreshTick;

final historySessionsProvider = FutureProvider<List<HistorySessionSummary>>((
  ref,
) async {
  ref.watch(historyRefreshTickProvider);
  final transport = await ref.watch(libraryTransportProvider.future);
  return transport.listHistorySessions();
});

final historyCanResumeProvider = FutureProvider<bool>((ref) async {
  ref.watch(historyRefreshTickProvider);
  final transport = await ref.watch(libraryTransportProvider.future);
  return transport.historyCanResume();
});

final selectedHistorySessionIdProvider =
    NotifierProvider<SelectedHistorySessionId, String?>(
      SelectedHistorySessionId.new,
    );

class SelectedHistorySessionId extends Notifier<String?> {
  @override
  String? build() => null;

  void set(String? id) => state = id;
}

final activeHistorySessionIdProvider = Provider<String?>((ref) {
  final selected = ref.watch(selectedHistorySessionIdProvider);
  final sessions = ref.watch(historySessionsProvider).asData?.value;
  if (sessions == null || sessions.isEmpty) {
    return null;
  }
  if (selected != null && sessions.any((s) => s.id == selected)) {
    return selected;
  }
  return sessions.first.id;
});

final historyEntriesProvider = FutureProvider<List<HistoryEntryInfo>>((
  ref,
) async {
  final sessionId = ref.watch(activeHistorySessionIdProvider);
  if (sessionId == null) {
    return const [];
  }
  ref.watch(historyRefreshTickProvider);
  final transport = await ref.watch(libraryTransportProvider.future);
  return transport.historySessionEntries(sessionId: sessionId);
});

/// Open (unclosed) history session, if any.
final openHistorySessionIdProvider = Provider<String?>((ref) {
  final sessions = ref.watch(historySessionsProvider).asData?.value;
  if (sessions == null) {
    return null;
  }
  for (final session in sessions) {
    if (!session.closed) {
      return session.id;
    }
  }
  return null;
});

/// Track ids / filesystem paths committed in the open history session.
class SessionPlayedKeys {
  const SessionPlayedKeys({required this.trackIds, required this.paths});

  final Set<String> trackIds;
  final Set<String> paths;

  static const empty = SessionPlayedKeys(trackIds: {}, paths: {});

  bool matches({String? trackId, String? path}) {
    if (trackId != null && trackId.isNotEmpty && trackIds.contains(trackId)) {
      return true;
    }
    if (path != null && path.isNotEmpty && paths.contains(path)) {
      return true;
    }
    return false;
  }
}

/// Convert history XSPF `location` (often `file://…`) to a filesystem path.
String normalizeHistoryLocation(String location) {
  if (location.startsWith('file://')) {
    try {
      return Uri.parse(location).toFilePath();
    } catch (_) {
      return location.substring('file://'.length);
    }
  }
  return location;
}

SessionPlayedKeys sessionPlayedKeysFromEntries(
  Iterable<HistoryEntryInfo> entries,
) {
  final trackIds = <String>{};
  final paths = <String>{};
  for (final entry in entries) {
    final id = entry.trackId;
    if (id != null && id.isNotEmpty) {
      trackIds.add(id);
    }
    final path = normalizeHistoryLocation(entry.location);
    if (path.isNotEmpty) {
      paths.add(path);
    }
  }
  return SessionPlayedKeys(trackIds: trackIds, paths: paths);
}

/// Keys for rows to dim when Settings → Library → Dim played tracks is on.
final sessionPlayedKeysProvider = FutureProvider<SessionPlayedKeys>((
  ref,
) async {
  final settings = ref.watch(appSettingsProvider).asData?.value;
  if (settings == null ||
      !settings.historyEnabled ||
      !settings.dimPlayedTracks) {
    return SessionPlayedKeys.empty;
  }
  ref.watch(historyRefreshTickProvider);
  final sessionId = ref.watch(openHistorySessionIdProvider);
  if (sessionId == null) {
    return SessionPlayedKeys.empty;
  }
  final transport = await ref.watch(libraryTransportProvider.future);
  final entries = await transport.historySessionEntries(sessionId: sessionId);
  return sessionPlayedKeysFromEntries(entries);
});

class HistoryEntryFilter extends Notifier<String> {
  @override
  String build() => '';

  void set(String value) => state = value;
}

final historyEntryFilterProvider = NotifierProvider<HistoryEntryFilter, String>(
  HistoryEntryFilter.new,
);

String historyEntryDisplayTitle(HistoryEntryInfo entry) {
  if (entry.title?.isNotEmpty ?? false) {
    return entry.title!;
  }
  final slash = entry.location.lastIndexOf('/');
  return slash >= 0 ? entry.location.substring(slash + 1) : entry.location;
}

bool historyEntryMatchesFilter(HistoryEntryInfo entry, String filter) {
  if (filter.isEmpty) {
    return true;
  }
  final haystack = [
    historyEntryDisplayTitle(entry),
    entry.artist ?? '',
    entry.album ?? '',
    entry.key ?? '',
    entry.isrc ?? '',
    deckDisplayLabel(entry.deck),
    fileNameFromPath(entry.location),
    entry.location,
  ].join('\n').toLowerCase();
  return haystack.contains(filter);
}

final filteredHistoryEntriesProvider =
    Provider<AsyncValue<List<HistoryEntryInfo>>>((ref) {
      final filter = ref.watch(historyEntryFilterProvider).trim().toLowerCase();
      final entries = ref.watch(historyEntriesProvider);
      return entries.whenData((list) {
        if (filter.isEmpty) {
          return list;
        }
        return [
          for (final entry in list)
            if (historyEntryMatchesFilter(entry, filter)) entry,
        ];
      });
    });

void invalidateHistory(WidgetRef ref) {
  ref.read(historyRefreshTickProvider.notifier).bump();
  ref.invalidate(collectionsProvider);
}

/// Apply history settings once settings + library transports are ready.
final historySettingsBootstrapProvider = Provider<void>((ref) {
  final settings = ref.watch(appSettingsProvider);
  final library = ref.watch(libraryTransportProvider);
  if (settings is AsyncData<AppSettings> &&
      library is AsyncData<LibraryTransport>) {
    unawaited(
      library.value.applyHistorySettings(
        enabled: settings.value.historyEnabled,
        sessionIdleMinutes: settings.value.historySessionIdleMinutes,
        minPlaySeconds: settings.value.historyMinPlaySeconds,
        minDeckVolume: settings.value.historyMinDeckVolume,
      ),
    );
  }
});

String formatHistoryTimestamp(String iso) {
  final parsed = DateTime.tryParse(iso);
  if (parsed != null) {
    final local = parsed.toLocal();
    final y = local.year.toString().padLeft(4, '0');
    final m = local.month.toString().padLeft(2, '0');
    final d = local.day.toString().padLeft(2, '0');
    final h = local.hour.toString().padLeft(2, '0');
    final min = local.minute.toString().padLeft(2, '0');
    return '$y-$m-$d $h:$min';
  }
  if (iso.length >= 16) {
    return iso.substring(0, 16).replaceFirst('T', ' ');
  }
  return iso;
}

String formatPlayedDurationMs(int? ms) {
  if (ms == null || ms <= 0) {
    return '—';
  }
  final totalSec = ms ~/ 1000;
  final m = totalSec ~/ 60;
  final s = totalSec % 60;
  return '$m:${s.toString().padLeft(2, '0')}';
}
