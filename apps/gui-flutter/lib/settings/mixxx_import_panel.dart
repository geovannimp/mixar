import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/settings/settings_widgets.dart';
import 'package:gui_flutter/shell/app_button.dart';
import 'package:gui_flutter/shell/mixar_dialog.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:gui_flutter/shell/mixar_toast.dart';
import 'package:gui_flutter/src/rust/api/library.dart';

/// Path to the user's Mixxx database, or `null` when none is found.
///
/// The adapter probes the platform's default Mixxx location. A failed probe
/// (e.g. no host bridge) surfaces as an error so the UI can distinguish it from
/// "not installed".
final mixxxDatabasePathProvider = FutureProvider<String?>((ref) async {
  return await mixxxDefaultDatabasePath();
});

/// Settings → Library: import tracks, playlists, and crates from Mixxx.
///
/// The database is auto-detected; the action stays disabled until one is found.
class MixxxImportPanel extends ConsumerStatefulWidget {
  const new({super.key});

  @override
  ConsumerState<MixxxImportPanel> createState() => _MixxxImportPanelState();
}

class _MixxxImportPanelState extends ConsumerState<MixxxImportPanel> {
  var _busy = false;

  Future<void> _run(String path) async {
    setState(() => _busy = true);
    try {
      final MixxxImportPreview preview;
      try {
        preview = await mixxxImportPreview(dbPath: path);
      } on Object catch (e) {
        if (mounted) {
          _toastError(
            AppLocalizations.of(context)!.mixxxImportReadFailed,
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
        title: l10n.mixxxImportConfirmTitle,
        body: l10n.mixxxImportConfirmBody(
          path,
          preview.trackCount,
          preview.missingFileCount,
          preview.playlistCount,
          preview.crateCount,
          preview.folderCount,
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

      final transport = await ref.read(libraryTransportProvider.future);
      final report = await transport.importMixxxLibrary(dbPath: path);
      if (!mounted) {
        return;
      }
      ref
        ..invalidate(collectionsProvider)
        ..invalidate(collectionTracksProvider);
      _toastReport(report);
    } on Object catch (e) {
      if (mounted) {
        _toastError(AppLocalizations.of(context)!.mixxxImportFailed, e);
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  void _toastReport(MixxxImportReport report) {
    final l10n = AppLocalizations.of(context)!;
    if (report.failed > 0) {
      final details = report.errors.take(3).join('\n');
      showMixarToast(
        context: context,
        title: Text(l10n.mixxxImportFinishedWithErrors(report.failed)),
        description: Text(
          details.isEmpty ? mixxxImportSummary(l10n, report) : details,
        ),
        variant: MixarToastVariant.destructive,
      );
      return;
    }
    showMixarToast(
      context: context,
      title: Text(mixxxImportSummary(l10n, report)),
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
    final database = ref.watch(mixxxDatabasePathProvider);
    final dbPath = database.asData?.value;
    final status = dbPath != null
        ? l10n.mixxxImportFound(dbPath)
        : database.hasError
        ? l10n.mixxxImportCheckFailed
        : database.isLoading
        ? l10n.mixxxImportLooking
        : l10n.mixxxImportNotFound;
    final onPress = !_busy && dbPath != null ? () => _run(dbPath) : null;

    return SettingsPanel(
      child: Row(
        spacing: 16,
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              spacing: 4,
              children: [
                Text(
                  l10n.mixxxImportTitle,
                  style: theme.typography.body.sm.copyWith(
                    fontWeight: FontWeight.w600,
                  ),
                ),
                Text(
                  l10n.mixxxImportDescription,
                  style: theme.typography.body.sm.copyWith(
                    color: theme.colors.mutedForeground,
                  ),
                ),
                Text(
                  status,
                  style: theme.typography.body.xs.copyWith(
                    color: theme.colors.mutedForeground,
                  ),
                ),
              ],
            ),
          ),
          AppButton(
            size: MixarButtonSize.sm,
            onPress: onPress,
            child: Text(
              _busy ? l10n.mixxxImporting : l10n.mixxxImportButton,
            ),
          ),
        ],
      ),
    );
  }
}

/// One-line result of a Mixxx import, shown in the completion toast.
String mixxxImportSummary(AppLocalizations l10n, MixxxImportReport report) {
  final imported =
      report.tracksAdded +
      report.playlistsImported +
      report.cratesImported +
      report.foldersImported;
  final missing = report.tracksMissingFiles > 0
      ? l10n.mixxxImportMissingSuffix(report.tracksMissingFiles)
      : '';
  if (imported == 0) {
    if (report.tracksUpdated == 0) {
      return report.collectionsSkipped > 0
          ? l10n.mixxxImportAlreadyImported
          : l10n.mixxxImportNothing;
    }
    final updated = l10n.mixxxCountTracks(report.tracksUpdated);
    return l10n.mixxxImportUpdated(updated, missing);
  }
  final parts = [
    if (report.tracksAdded > 0) l10n.mixxxCountTracks(report.tracksAdded),
    if (report.tracksUpdated > 0) l10n.mixxxCountUpdated(report.tracksUpdated),
    if (report.playlistsImported > 0)
      l10n.mixxxCountPlaylists(report.playlistsImported),
    if (report.cratesImported > 0) l10n.mixxxCountCrates(report.cratesImported),
    if (report.foldersImported > 0)
      l10n.mixxxCountFolders(report.foldersImported),
  ];
  return l10n.mixxxImportImported(parts.join(', '), missing);
}
