import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/mixer/fader_slider.dart';
import 'package:gui_flutter/mixer/track_drag.dart';
import 'package:gui_flutter/settings/settings_providers.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';

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

final historyEntriesProvider = FutureProvider<List<HistoryEntryInfo>>((ref) async {
  final sessionId = ref.watch(activeHistorySessionIdProvider);
  if (sessionId == null) {
    return const [];
  }
  ref.watch(historyRefreshTickProvider);
  final transport = await ref.watch(libraryTransportProvider.future);
  return transport.historySessionEntries(sessionId: sessionId);
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

class HistoryRefreshTick extends Notifier<int> {
  @override
  int build() => 0;

  void bump() => state++;
}

final historyRefreshTickProvider =
    NotifierProvider<HistoryRefreshTick, int>(HistoryRefreshTick.new);

void invalidateHistory(WidgetRef ref) {
  ref.read(historyRefreshTickProvider.notifier).bump();
  ref.invalidate(collectionsProvider);
}

/// Refresh session list while an open session may close on idle timeout.
final historyLivePollProvider = Provider<void>((ref) {
  if (ref.watch(librarySourceTabProvider) != LibrarySourceTab.history) {
    return;
  }
  final sessions = ref.watch(historySessionsProvider);
  final hasLive = sessions.asData?.value.any((s) => !s.closed) ?? false;
  if (!hasLive) {
    return;
  }
  final timer = Timer.periodic(const Duration(seconds: 2), (_) {
    ref.read(historyRefreshTickProvider.notifier).bump();
  });
  ref.onDispose(timer.cancel);
});

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
