import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/theme/mixar_button_style.dart';
import 'package:gui_flutter/core/utils/settings_search.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/providers/library/deck_import_refresh.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/widgets/settings/settings_searchable_card.dart';
import 'package:gui_flutter/widgets/settings/settings_section_card.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/mixar_dialog.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:gui_flutter/widgets/shell/mixar_toast.dart';

/// Path to the user's Rekordbox `master.db`, or `null` when none is found.
///
/// The adapter probes the platform's default Rekordbox location. A failed probe
/// (e.g. no host bridge) surfaces as an error so the UI can distinguish it from
/// "not installed".
final rekordboxDatabasePathProvider = FutureProvider<String?>((ref) async {
  return await rekordboxDefaultDatabasePath();
});

/// Settings → Library: import tracks, playlists, hot cues, and loops from
/// Rekordbox 6/7.
///
/// The database is auto-detected; the action stays disabled until one is found.
/// The database stays read-only: the host unlocks a private copy of it.
class RekordboxImportPanel extends ConsumerStatefulWidget
    implements SettingsSearchableCard {
  const new({this.decorated = true, super.key});

  /// Band chrome; setup steps turn it off.
  final bool decorated;

  @override
  bool matchesSettingsSearch(BuildContext context, String foldedQuery) {
    final l10n = AppLocalizations.of(context)!;
    return settingsSearchMatches(
      '${l10n.rekordboxImportTitle} ${l10n.rekordboxImportDescription}',
      foldedQuery,
    );
  }

  @override
  ConsumerState<RekordboxImportPanel> createState() =>
      _RekordboxImportPanelState();
}

class _RekordboxImportPanelState extends ConsumerState<RekordboxImportPanel> {
  var _busy = false;
  var _importing = false;

  Future<void> _run(String path) async {
    setState(() => _busy = true);
    try {
      final RekordboxImportPreview preview;
      try {
        preview = await rekordboxImportPreview(dbPath: path);
      } on Object catch (e) {
        if (mounted) {
          _toastError(
            AppLocalizations.of(context)!.rekordboxImportReadFailed,
            e,
          );
        }
        return;
      }
      if (!mounted) {
        return;
      }

      final l10n = AppLocalizations.of(context)!;
      final confirmed = await showMixarConfirm<bool>(
        context: context,
        title: l10n.rekordboxImportConfirmTitle,
        body: l10n.rekordboxImportConfirmBody(
          path,
          preview.trackCount,
          preview.missingFileCount,
          preview.playlistCount,
          preview.smartPlaylistCount,
          preview.folderCount,
          preview.hotCueCount,
          preview.loopCount,
        ),
        actions: [
          MixarDialogAction(
            label: l10n.settingsCancel,
            value: false,
            variant: MixarButtonVariant.outline,
          ),
          MixarDialogAction(label: l10n.commonImport, value: true),
        ],
      );
      if (confirmed != true || !mounted) {
        return;
      }
      setState(() => _importing = true);

      final transport = await ref.read(libraryTransportProvider.future);
      final report = await transport.importRekordboxLibrary(dbPath: path);
      if (!mounted) {
        return;
      }
      ref
        ..invalidate(collectionsProvider)
        ..invalidate(collectionTracksProvider);
      // The import wrote cues and loops behind any track already on a deck.
      await refreshLoadedDeckTracks(ref);
      if (!mounted) {
        return;
      }
      _toastReport(report);
    } on Object catch (e) {
      if (mounted) {
        _toastError(AppLocalizations.of(context)!.rekordboxImportFailed, e);
      }
    } finally {
      if (mounted) {
        setState(() {
          _busy = false;
          _importing = false;
        });
      }
    }
  }

  void _toastReport(RekordboxImportReport report) {
    final l10n = AppLocalizations.of(context)!;
    if (report.failed > 0) {
      final details = report.errors.take(3).join('\n');
      showMixarToast(
        context: context,
        title: Text(l10n.rekordboxImportFinishedWithErrors(report.failed)),
        description: Text(
          details.isEmpty ? rekordboxImportSummary(l10n, report) : details,
        ),
        variant: MixarToastVariant.destructive,
      );
      return;
    }
    showMixarToast(
      context: context,
      title: Text(rekordboxImportSummary(l10n, report)),
      variant: MixarToastVariant.success,
    );
  }

  void _toastError(String title, Object error) {
    showMixarToast(
      context: context,
      title: Text(title),
      description: Text('$error'),
      variant: MixarToastVariant.destructive,
    );
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final l10n = AppLocalizations.of(context)!;
    final database = ref.watch(rekordboxDatabasePathProvider);
    final dbPath = database.asData?.value;
    final status = dbPath != null
        ? l10n.rekordboxImportFound(dbPath)
        : database.hasError
        ? l10n.rekordboxImportCheckFailed
        : database.isLoading
        ? l10n.rekordboxImportLooking
        : l10n.rekordboxImportNotFound;
    final onPress = !_busy && dbPath != null ? () => _run(dbPath) : null;

    return SettingsSectionCard(
      decorated: widget.decorated,
      title: l10n.rekordboxImportTitle,
      subtitle: l10n.rekordboxImportDescription,
      trailing: AppButton(
        size: MixarButtonSize.sm,
        onPress: onPress,
        child: Text(
          _importing ? l10n.rekordboxImporting : l10n.rekordboxImportButton,
        ),
      ),
      child: Text(
        status,
        style: theme.typography.body.xs.copyWith(
          color: theme.colors.mutedForeground,
        ),
      ),
    );
  }
}

/// One-line result of a Rekordbox import, shown in the completion toast.
String rekordboxImportSummary(
  AppLocalizations l10n,
  RekordboxImportReport report,
) {
  final imported = report.tracksAdded + report.playlistsImported;
  final missing = report.tracksMissingFiles > 0
      ? l10n.rekordboxImportMissingSuffix(report.tracksMissingFiles)
      : '';
  final cues = report.hotCuesImported + report.loopsImported;
  if (imported == 0 && cues == 0) {
    if (report.tracksUpdated == 0) {
      return report.collectionsSkipped > 0
          ? l10n.rekordboxImportAlreadyImported
          : l10n.rekordboxImportNothing;
    }
    final updated = l10n.rekordboxCountUpdated(report.tracksUpdated);
    return l10n.rekordboxImportUpdated(updated, missing);
  }
  final parts = [
    if (report.tracksAdded > 0) l10n.rekordboxCountTracks(report.tracksAdded),
    if (report.tracksUpdated > 0)
      l10n.rekordboxCountUpdated(report.tracksUpdated),
    if (report.playlistsImported > 0)
      l10n.rekordboxCountPlaylists(report.playlistsImported),
    if (report.hotCuesImported > 0)
      l10n.rekordboxCountHotCues(report.hotCuesImported),
    if (report.loopsImported > 0)
      l10n.rekordboxCountLoops(report.loopsImported),
  ];
  return l10n.rekordboxImportImported(parts.join(', '), missing);
}
