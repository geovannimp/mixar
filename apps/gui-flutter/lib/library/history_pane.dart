import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/library/history_providers.dart';
import 'package:gui_flutter/library/library_nav.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/src/rust/api/library.dart';

/// Sidebar list of performance history sessions.
class HistoryPane extends ConsumerStatefulWidget {
  const HistoryPane({super.key});

  @override
  ConsumerState<HistoryPane> createState() => _HistoryPaneState();
}

class _HistoryPaneState extends ConsumerState<HistoryPane> {
  var _creating = false;

  @override
  Widget build(BuildContext context) {
    ref.watch(historyLivePollProvider);
    final theme = context.theme;
    final colors = theme.colors;
    final sessions = ref.watch(historySessionsProvider);
    final selectedId = ref.watch(activeHistorySessionIdProvider);
    final canResume = ref.watch(historyCanResumeProvider);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(10, 4, 4, 8),
          child: Row(
            children: [
              const Expanded(child: LibraryPaneLabel('History')),
              FTappable(
                onPress: _creating ? null : () => unawaited(_newSession()),
                semanticsLabel: 'New session',
                builder: (context, variants, _) {
                  final hovered = variants.contains(FTappableVariant.hovered);
                  return Container(
                    width: 24,
                    height: 24,
                    alignment: Alignment.center,
                    decoration: BoxDecoration(
                      borderRadius: theme.style.borderRadius.sm,
                      border: Border.all(
                        color: colors.primary.withValues(alpha: 0.35),
                      ),
                      color: colors.primary.withValues(
                        alpha: hovered ? 0.20 : 0.12,
                      ),
                    ),
                    child: _creating
                        ? const FCircularProgress(size: .sm)
                        : Text(
                            '+',
                            style: theme.typography.body.sm.copyWith(
                              color: colors.primary,
                              fontWeight: FontWeight.w600,
                            ),
                          ),
                  );
                },
              ),
            ],
          ),
        ),
        Expanded(
          child: sessions.when(
            loading: () => const Center(child: FCircularProgress()),
            error: (e, _) => Center(
              child: Padding(
                padding: const EdgeInsets.all(12),
                child: Text(
                  '$e',
                  style: theme.typography.body.sm.copyWith(
                    color: theme.colors.destructive,
                  ),
                ),
              ),
            ),
            data: (rows) {
              if (rows.isEmpty) {
                return Center(
                  child: Text(
                    'No sessions yet',
                    style: theme.typography.body.sm.copyWith(
                      color: theme.colors.mutedForeground,
                    ),
                  ),
                );
              }
              final resumeSessionId = canResume.maybeWhen(
                data: (value) => value ? _resumableSessionId(rows) : null,
                orElse: () => null,
              );
              return ListView.builder(
                itemCount: rows.length,
                itemBuilder: (context, index) {
                  final row = rows[index];
                  return LibraryNavRow(
                    title: row.title,
                    subtitle:
                        '${formatHistoryTimestamp(row.startedAt)} · ${row.entryCount} plays',
                    selected: row.id == selectedId,
                    onPress: () => ref
                        .read(selectedHistorySessionIdProvider.notifier)
                        .set(row.id),
                    trailing: row.id == resumeSessionId
                        ? FTappable(
                            semanticsLabel: 'Resume ${row.title}',
                            onPress: () => unawaited(_resumeSession()),
                            child: Padding(
                              padding: const EdgeInsets.all(4),
                              child: Icon(
                                FLucideIcons.iterationCw,
                                size: 14,
                                color: colors.mutedForeground,
                              ),
                            ),
                          )
                        : row.closed
                        ? null
                        : Padding(
                            padding: const EdgeInsets.only(right: 4),
                            child: _LiveBadge(theme: theme),
                          ),
                  );
                },
              );
            },
          ),
        ),
      ],
    );
  }

  static String? _resumableSessionId(List<HistorySessionSummary> rows) {
    for (final row in rows) {
      if (row.closed) {
        return row.id;
      }
    }
    return null;
  }

  Future<void> _newSession() async {
    setState(() => _creating = true);
    ref.read(libraryMessageProvider.notifier).clear();
    try {
      final transport = await ref.read(libraryTransportProvider.future);
      await transport.historyNewSession();
      invalidateHistory(ref);
    } catch (e) {
      if (mounted) {
        ref.read(libraryMessageProvider.notifier).setError('$e');
      }
    } finally {
      if (mounted) {
        setState(() => _creating = false);
      }
    }
  }

  Future<void> _resumeSession() async {
    ref.read(libraryMessageProvider.notifier).clear();
    try {
      final transport = await ref.read(libraryTransportProvider.future);
      await transport.historyResumeSession();
      invalidateHistory(ref);
    } catch (e) {
      if (mounted) {
        ref.read(libraryMessageProvider.notifier).setError('$e');
      }
    }
  }
}

class _LiveBadge extends StatelessWidget {
  const _LiveBadge({required this.theme});

  static const _liveGreen = Color(0xFF22C55E);

  final FThemeData theme;

  @override
  Widget build(BuildContext context) {
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        const DecoratedBox(
          decoration: BoxDecoration(
            color: _liveGreen,
            shape: BoxShape.circle,
          ),
          child: SizedBox(width: 6, height: 6),
        ),
        const SizedBox(width: 4),
        Text(
          'live',
          style: theme.typography.body.xs.copyWith(
            color: _liveGreen,
            fontWeight: FontWeight.w600,
          ),
        ),
      ],
    );
  }
}
