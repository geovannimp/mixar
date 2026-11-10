import 'package:file_picker/file_picker.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/utils/android_media_access.dart';
import 'package:gui_flutter/core/utils/collection_actions.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/widgets/onboarding/onboarding_ui.dart';
import 'package:gui_flutter/widgets/settings/mixxx_import_panel.dart';
import 'package:gui_flutter/widgets/settings/rekordbox_import_panel.dart';
import 'package:gui_flutter/widgets/shell/m_loader.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:gui_flutter/widgets/shell/mixar_toast.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Step 4: point Mixar at music, or import a library another DJ app already
/// built. Everything goes through the library transport, so it works before
/// the engine exists — first run and a replay both.
class OnboardingLibraryStep extends ConsumerStatefulWidget {
  const new({super.key});

  @override
  ConsumerState<OnboardingLibraryStep> createState() =>
      _OnboardingLibraryStepState();
}

class _OnboardingLibraryStepState extends ConsumerState<OnboardingLibraryStep> {
  bool _busy = false;
  String? _added;

  Future<void> _chooseFolder() async {
    // One import at a time: permission prompt and picker are both async, and a
    // second tap would open a second permission request the host rejects.
    if (_busy) {
      return;
    }
    setState(() => _busy = true);
    final l10n = AppLocalizations.of(context)!;
    try {
      // Ask before the picker: without the Android media permission the host
      // scan cannot see the files, so the folder would import as empty.
      if (!await ensureAudioReadAccess()) {
        _showAddFolderFailure(const Text(audioReadDeniedMessage));
        return;
      }
      final path = await FilePicker.getDirectoryPath(
        dialogTitle: l10n.onboardingLibraryChooseFolder,
      );
      if (path == null || !mounted) {
        return;
      }
      final library = await ref.read(libraryTransportProvider.future);
      // No name: the host derives the collection name from the folder.
      final result = await library.addFolderCollection(
        folderPath: path,
        scanFolderTree: true,
      );
      // The import can outlive the step (skip, back, or finish mid-scan), and
      // `ref` throws once the state is disposed.
      if (!mounted) {
        return;
      }
      // On a replay the mixer's providers are already alive, so the sidebar
      // would keep showing the old counts without this.
      refreshCollectionViews(ref);
      // Only a folder that brought tracks in counts as added: calling an empty
      // or unreadable folder "added" would report failure as success.
      final notice = folderImportNotice(result);
      final imported = result.added + result.updated > 0;
      if (imported) {
        setState(() => _added = result.collection.name);
      }
      showMixarToast(
        context: context,
        title: Text(
          imported
              ? l10n.onboardingLibraryFolderAdded
              : l10n.onboardingLibraryFolderFailed,
        ),
        description: notice == null ? null : Text(notice.message),
        variant: imported && notice == null
            ? MixarToastVariant.success
            : MixarToastVariant.destructive,
      );
    } on Object catch (e) {
      _showAddFolderFailure(Text('$e'));
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  /// Destructive toast for anything that kept a folder from being added.
  void _showAddFolderFailure(Widget description) {
    if (!mounted) {
      return;
    }
    showMixarToast(
      context: context,
      title: Text(AppLocalizations.of(context)!.onboardingLibraryFolderFailed),
      description: description,
      variant: MixarToastVariant.destructive,
    );
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final theme = context.theme;
    // Only libraries that exist on this machine get a row; the rest are named
    // once underneath, so there are no dead buttons to wonder about.
    final mixxxProbe = ref.watch(mixxxDatabasePathProvider);
    final rekordboxProbe = ref.watch(rekordboxDatabasePathProvider);
    // A probe that is still running, or that failed, knows nothing: claiming
    // "not found" then would be wrong, and the panels say the same thing.
    final resolved =
        mixxxProbe.hasValue &&
        rekordboxProbe.hasValue &&
        !mixxxProbe.isLoading &&
        !rekordboxProbe.isLoading;
    final mixxx = mixxxProbe.asData?.value;
    final rekordbox = rekordboxProbe.asData?.value;
    // Which library is missing decides the sentence. The brand names live next
    // to the copy that renders them, in one call each.
    final missingCopy = !resolved
        ? null
        : switch ((mixxx == null, rekordbox == null)) {
            (true, true) => l10n.onboardingLibraryNotFoundBoth,
            (true, false) => l10n.onboardingLibraryNotFoundOne('Mixxx'),
            (false, true) => l10n.onboardingLibraryNotFoundOne('Rekordbox'),
            _ => null,
          };

    return OnboardingBody(
      eyebrow: l10n.onboardingKickerLibrary,
      title: l10n.onboardingLibraryTitle,
      description: l10n.onboardingLibraryBody,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        spacing: 24,
        children: [
          _FolderCard(
            busy: _busy,
            added: _added,
            onPress: _busy ? null : _chooseFolder,
          ),
          Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            spacing: 4,
            children: [
              OnboardingGroupLabel(label: l10n.onboardingLibraryImportLabel),
              if (mixxx != null) const MixxxImportPanel(decorated: false),
              if (rekordbox != null)
                const RekordboxImportPanel(decorated: false),
              if (missingCopy case final missing?)
                Padding(
                  padding: const EdgeInsets.only(top: 10),
                  child: Text(
                    missing,
                    style: theme.typography.body.sm.copyWith(
                      color: theme.colors.mutedForeground,
                      height: 1.45,
                    ),
                  ),
                ),
            ],
          ),
        ],
      ),
    );
  }
}

/// Primary action: the folder chooser, with the result kept on screen so the
/// step does not look like nothing happened.
class _FolderCard extends StatelessWidget {
  const new({required this.busy, required this.added, required this.onPress});

  final bool busy;
  final String? added;
  final VoidCallback? onPress;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final l10n = AppLocalizations.of(context)!;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: theme.colors.card,
        border: Border.all(color: theme.colors.primary, width: 2),
      ),
      child: Padding(
        padding: const EdgeInsets.all(20),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          spacing: 16,
          children: [
            Icon(LucideIcons.folderOpen, size: 22, color: theme.colors.primary),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                spacing: 6,
                children: [
                  Text(
                    l10n.onboardingLibraryChooseFolder,
                    style: theme.typography.body.lg.copyWith(
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  Text(
                    added == null
                        ? l10n.onboardingLibraryFolderHint
                        : l10n.onboardingLibraryFolderReady(added!),
                    style: theme.typography.body.sm.copyWith(
                      color: theme.colors.mutedForeground,
                      height: 1.45,
                    ),
                  ),
                ],
              ),
            ),
            if (busy)
              const Padding(
                padding: EdgeInsets.all(12),
                child: MLoader(size: MLoaderSize.sm),
              )
            else
              _BrowseButton(onPress: onPress),
          ],
        ),
      ),
    );
  }
}

class _BrowseButton extends StatelessWidget {
  const new({required this.onPress});

  final VoidCallback? onPress;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return OnboardingSmallButton(
      onPress: onPress,
      label: l10n.onboardingLibraryBrowse,
      icon: LucideIcons.arrowRight,
    );
  }
}
