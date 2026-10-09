import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/settings/settings_widgets.dart';
import 'package:gui_flutter/shell/app_button.dart';
import 'package:gui_flutter/shell/mixar_dialog.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:gui_flutter/shell/mixar_toast.dart';
import 'package:gui_flutter/src/rust/api/library.dart';

/// Path to the user's Mixxx database, or `null` when none is found.
///
/// The adapter probes the platform's default Mixxx location; a failure to
/// resolve (e.g. no host bridge in tests) degrades to "not found" rather than
/// surfacing an error.
final mixxxDatabasePathProvider = FutureProvider<String?>((ref) async {
  try {
    return await mixxxDefaultDatabasePath();
  } on Object {
    return null;
  }
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
    final MixxxImportPreview preview;
    try {
      preview = await mixxxImportPreview(dbPath: path);
    } on Object catch (e) {
      if (mounted) {
        _toastError('Could not read Mixxx library', e);
      }
      return;
    }
    if (!mounted) {
      return;
    }

    final confirmed = await showMixarConfirm<bool>(
      context: context,
      title: 'Import from Mixxx?',
      body:
          'This imports tracks, playlists, crates, and watched folders from:\n'
          '$path\n\n'
          '${preview.trackCount} tracks '
          '(${preview.missingFileCount} missing), '
          '${preview.playlistCount} playlists, '
          '${preview.crateCount} crates, '
          '${preview.folderCount} folders.\n\n'
          'Missing files are imported as unavailable tracks.',
      actions: const [
        MixarDialogAction(
          label: 'Cancel',
          value: false,
          variant: MixarButtonVariant.outline,
        ),
        MixarDialogAction(label: 'Import', value: true),
      ],
    );
    if (confirmed != true || !mounted) {
      return;
    }

    setState(() => _busy = true);
    try {
      final transport = await ref.read(libraryTransportProvider.future);
      final report = await transport.importMixxxLibrary(dbPath: path);
      ref
        ..invalidate(collectionsProvider)
        ..invalidate(collectionTracksProvider);
      if (!mounted) {
        return;
      }
      showMixarToast(context: context, title: Text(_summary(report)));
    } on Object catch (e) {
      if (mounted) {
        _toastError('Mixxx import failed', e);
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  String _summary(MixxxImportReport report) {
    final imported =
        report.tracksAdded +
        report.playlistsImported +
        report.cratesImported +
        report.foldersImported;
    if (imported == 0 && report.collectionsSkipped > 0) {
      return 'Mixxx library already imported';
    }
    final parts = [
      if (report.tracksAdded > 0) '${report.tracksAdded} tracks',
      if (report.playlistsImported > 0) '${report.playlistsImported} playlists',
      if (report.cratesImported > 0) '${report.cratesImported} crates',
      if (report.foldersImported > 0) '${report.foldersImported} folders',
    ];
    if (parts.isEmpty) {
      return 'Nothing new to import from Mixxx';
    }
    return 'Imported ${parts.join(', ')} from Mixxx';
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
    final database = ref.watch(mixxxDatabasePathProvider);
    final dbPath = database.asData?.value;
    final status = dbPath != null
        ? 'Found: $dbPath'
        : database.isLoading
        ? 'Looking for a Mixxx library…'
        : 'No Mixxx library found on this computer.';
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
                  'Import from Mixxx',
                  style: theme.typography.body.sm.copyWith(
                    fontWeight: FontWeight.w600,
                  ),
                ),
                Text(
                  'Bring tracks, playlists, and crates from your Mixxx '
                  'library into Mixar.',
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
            child: Text(_busy ? 'Importing…' : 'Import from Mixxx library…'),
          ),
        ],
      ),
    );
  }
}
