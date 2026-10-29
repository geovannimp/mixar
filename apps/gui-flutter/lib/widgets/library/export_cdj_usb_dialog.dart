import 'package:file_picker/file_picker.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/theme/mixar_button_style.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/widgets/settings/settings_field.dart';
import 'package:gui_flutter/widgets/settings/settings_storage_panel.dart'
    show formatStorageBytes;
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/mixar_checkbox.dart';
import 'package:gui_flutter/widgets/shell/mixar_dialog.dart';
import 'package:gui_flutter/widgets/shell/mixar_input.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:gui_flutter/widgets/shell/mixar_toast.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Collection kind of a folder collection in the collection summary (`kind`).
const _folderKind = 'folder';

/// Native directory picker the dialog opens, kept behind a provider so tests
/// can drive the flow without the platform plugin.
final cdjDirectoryPickerProvider =
    Provider<Future<String?> Function(String? dialogTitle)>(
      (ref) =>
          (dialogTitle) =>
              FilePicker.getDirectoryPath(dialogTitle: dialogTitle),
    );

/// Export one collection to a CDJ/XDJ USB drive.
///
/// Opened from the collection's three-dot menu: the drive is chosen through the
/// platform's own directory picker, and the export runs while the dialog stays
/// up, so a long write reports its progress state and its result where the user
/// started it.
Future<void> showExportCdjUsbDialog(
  BuildContext context, {
  required LibraryCollectionSummary collection,
}) {
  return showMixarDialog<void>(
    context: context,
    builder: (_) => _ExportCdjUsbDialogBody(collection: collection),
  );
}

class _ExportCdjUsbDialogBody extends ConsumerStatefulWidget {
  const new({required this.collection});

  final LibraryCollectionSummary collection;

  @override
  ConsumerState<_ExportCdjUsbDialogBody> createState() =>
      _ExportCdjUsbDialogBodyState();
}

class _ExportCdjUsbDialogBodyState
    extends ConsumerState<_ExportCdjUsbDialogBody> {
  final _destination = TextEditingController();
  var _refreshFiles = false;
  var _busy = false;

  @override
  void dispose() {
    _destination.dispose();
    super.dispose();
  }

  String get _destinationPath => _destination.text.trim();

  bool get _canExport => !_busy && _destinationPath.isNotEmpty;

  /// Pick the destination through the platform's directory picker.
  Future<void> _browse() async {
    final l10n = AppLocalizations.of(context)!;
    final pickDirectory = ref.read(cdjDirectoryPickerProvider);
    final path = await pickDirectory(l10n.cdjExportTitle);
    if (path == null || !mounted) {
      return;
    }
    setState(() => _destination.text = path);
  }

  Future<void> _export() async {
    if (!_canExport) {
      return;
    }
    final l10n = AppLocalizations.of(context)!;
    final destination = _destinationPath;
    final collection = widget.collection;
    final isFolder = collection.kind == _folderKind;

    setState(() => _busy = true);
    try {
      final transport = await ref.read(libraryTransportProvider.future);
      final report = await transport.exportCdjUsb(
        destination: destination,
        playlists: isFolder ? const [] : [collection.id],
        folders: isFolder ? [collection.id] : const [],
        refreshFiles: _refreshFiles,
      );
      if (!mounted) {
        return;
      }
      _toastReport(report);
      Navigator.of(context).pop();
    } on Object catch (e) {
      if (mounted) {
        _toastError(l10n.cdjExportFailed, e);
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  void _toastReport(CdjExportReport report) {
    final l10n = AppLocalizations.of(context)!;
    final summary = cdjExportSummary(l10n, report);
    if (report.errors.isNotEmpty) {
      showMixarToast(
        context: context,
        title: Text(l10n.cdjExportFinishedWithErrors(report.errors.length)),
        description: Text(report.errors.take(3).join('\n')),
        variant: MixarToastVariant.destructive,
      );
      return;
    }
    if (report.warnings.isNotEmpty) {
      // The run went through, so the warning keeps the counts it wrote.
      showMixarToast(
        context: context,
        title: Text(l10n.cdjExportFinishedWithWarnings(report.warnings.length)),
        description: Text([...report.warnings.take(3), summary].join('\n')),
        variant: MixarToastVariant.destructive,
      );
      return;
    }
    showMixarToast(context: context, title: Text(summary));
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
    final collection = widget.collection;
    final isFolder = collection.kind == _folderKind;

    return Padding(
      padding: const EdgeInsets.all(16),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        spacing: 16,
        children: [
          Text(
            l10n.cdjExportTitle,
            style: theme.typography.body.md.copyWith(
              fontWeight: FontWeight.w700,
            ),
          ),
          Text(
            l10n.cdjExportDescription,
            style: theme.typography.body.sm.copyWith(
              color: theme.colors.mutedForeground,
            ),
          ),
          SettingsField(
            label: l10n.cdjExportCollection,
            child: Row(
              spacing: 6,
              children: [
                Icon(
                  isFolder ? LucideIcons.folder : LucideIcons.listMusic,
                  size: 14,
                  color: theme.colors.mutedForeground,
                ),
                Flexible(
                  child: Text(
                    collection.name,
                    overflow: TextOverflow.ellipsis,
                    style: theme.typography.body.sm,
                  ),
                ),
                // Smart playlists resolve their membership at export time, so
                // there is no count to show here.
                if (collection.trackCount case final count?)
                  Text(
                    '· ${l10n.cdjCountTracks(count)}',
                    style: theme.typography.body.xs.copyWith(
                      color: theme.colors.mutedForeground,
                    ),
                  ),
              ],
            ),
          ),
          SettingsField(
            label: l10n.cdjExportDestination,
            hint: _destinationPath.isEmpty
                ? l10n.cdjExportDestinationMissing
                : null,
            child: IntrinsicHeight(
              child: Row(
                spacing: 8,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Expanded(
                    child: MixarInput(
                      hint: l10n.cdjExportDestinationHint,
                      controller: _destination,
                      enabled: !_busy,
                      onChanged: (_) => setState(() {}),
                    ),
                  ),
                  AppButton(
                    size: MixarButtonSize.sm,
                    variant: MixarButtonVariant.outline,
                    onPress: _busy ? null : _browse,
                    child: Text(l10n.cdjExportBrowse),
                  ),
                ],
              ),
            ),
          ),
          MixarCheckbox(
            value: _refreshFiles,
            enabled: !_busy,
            onChanged: (value) => setState(() => _refreshFiles = value),
            label: Text(
              l10n.cdjExportOverride,
              style: theme.typography.body.sm.copyWith(
                color: theme.colors.foreground,
              ),
            ),
          ),
          Row(
            mainAxisAlignment: MainAxisAlignment.end,
            spacing: 8,
            children: [
              AppButton(
                variant: MixarButtonVariant.outline,
                onPress: _busy ? null : () => Navigator.of(context).pop(),
                child: Text(l10n.settingsCancel),
              ),
              AppButton(
                onPress: _canExport ? _export : null,
                child: Text(
                  _busy ? l10n.cdjExporting : l10n.cdjExportConfirmAction,
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

/// Toast line for a finished run: what it wrote, or that it wrote nothing.
String cdjExportSummary(AppLocalizations l10n, CdjExportReport report) {
  final parts = [
    if (report.tracks > 0) l10n.cdjCountTracks(report.tracks),
    if (report.tracksReused > 0) l10n.cdjCountReused(report.tracksReused),
    if (report.playlists > 0) l10n.cdjCountPlaylists(report.playlists),
    if (report.folders > 0) l10n.cdjCountFolders(report.folders),
    if (report.bytesCopied > BigInt.zero)
      l10n.cdjCountCopied(formatStorageBytes(report.bytesCopied)),
  ];
  if (parts.isEmpty) {
    return l10n.cdjExportNothing;
  }
  return l10n.cdjExportExported(parts.join(', '));
}
