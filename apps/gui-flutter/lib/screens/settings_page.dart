import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/models/settings_section.dart';
import 'package:gui_flutter/core/theme/mixar_button_style.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/providers/shell/controller_providers.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_audio_panel.dart';
import 'package:gui_flutter/widgets/settings/settings_controllers_panel.dart';
import 'package:gui_flutter/widgets/settings/settings_deck_panel.dart';
import 'package:gui_flutter/widgets/settings/settings_library_panel.dart';
import 'package:gui_flutter/widgets/settings/settings_mixer_panel.dart';
import 'package:gui_flutter/widgets/settings/settings_privacy_panel.dart';
import 'package:gui_flutter/widgets/settings/settings_session_panel.dart';
import 'package:gui_flutter/widgets/settings/settings_sidebar.dart';
import 'package:gui_flutter/widgets/settings/settings_storage_panel.dart';
import 'package:gui_flutter/widgets/settings/settings_ui_panel.dart';
import 'package:gui_flutter/widgets/settings/settings_waveform_panel.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/mixar_dialog.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:gui_flutter/widgets/shell/mixar_toast.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

class SettingsPage extends ConsumerStatefulWidget {
  const new({
    this.onClose,
    this.initialSection = SettingsSection.ui,
    this.scrollToPreviewBus = false,
    super.key,
  }) : assert(
         !scrollToPreviewBus || initialSection == SettingsSection.audio,
         'scrollToPreviewBus requires initialSection.audio',
       );

  final VoidCallback? onClose;
  final SettingsSection initialSection;

  /// Scroll + glow the Preview bus card. Requires [initialSection] audio.
  ///
  /// Remount the page (e.g. new [Key]) to apply a changed
  /// [initialSection]/[scrollToPreviewBus]; State does not sync them in
  /// [State.didUpdateWidget].
  final bool scrollToPreviewBus;

  @override
  ConsumerState<SettingsPage> createState() => _SettingsPageState();
}

class _SettingsPageState extends ConsumerState<SettingsPage> {
  late SettingsSection _section = widget.initialSection;
  late var _highlightPreviewBus = widget.scrollToPreviewBus;
  final _previewBusCardKey = GlobalKey();
  AppSettings? _draft;
  AppSettings? _baseline;
  var _busy = false;
  var _didSchedulePreviewScroll = false;
  var _previewScrollAttempts = 0;

  bool _isDirty(AppSettings draft, AppSettings baseline) =>
      appSettingsDirty(draft, baseline);

  @override
  void initState() {
    super.initState();
    if (widget.scrollToPreviewBus) {
      WidgetsBinding.instance.addPostFrameCallback((_) => _tryPreviewScroll());
    }
  }

  /// Post-frame poll until the Preview bus card is mounted (settings may still
  /// be loading on the first frame). Not called from [build].
  void _tryPreviewScroll() {
    if (!mounted ||
        _didSchedulePreviewScroll ||
        !widget.scrollToPreviewBus ||
        _section != SettingsSection.audio) {
      return;
    }
    final ctx = _previewBusCardKey.currentContext;
    if (ctx == null) {
      if (_previewScrollAttempts++ < 30) {
        WidgetsBinding.instance.addPostFrameCallback(
          (_) => _tryPreviewScroll(),
        );
      }
      return;
    }
    _didSchedulePreviewScroll = true;
    Scrollable.ensureVisible(
      ctx,
      alignment: 0.1,
      duration: const Duration(milliseconds: 200),
      curve: Curves.easeOut,
    );
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final l10n = AppLocalizations.of(context)!;
    final settingsAsync = ref.watch(appSettingsProvider);

    return settingsAsync.when(
      loading: () => Center(
        child: Text(
          l10n.settingsLoading,
          style: theme.typography.body.sm.copyWith(
            color: theme.colors.mutedForeground,
          ),
        ),
      ),
      error: (e, _) => Center(
        child: Text(
          '$e',
          style: theme.typography.body.sm.copyWith(
            color: theme.colors.destructive,
          ),
        ),
      ),
      data: (settings) {
        _baseline ??= settings;
        _draft ??= settings;
        final draft = _draft!;
        final baseline = _baseline!;
        final dirty = _isDirty(draft, baseline);
        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            DecoratedBox(
              decoration: BoxDecoration(
                border: Border(bottom: BorderSide(color: theme.colors.border)),
              ),
              child: Padding(
                padding: const EdgeInsets.fromLTRB(12, 12, 24, 12),
                child: Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  spacing: 12,
                  children: [
                    if (widget.onClose != null)
                      AppButton.icon(
                        variant: .ghost,
                        size: .sm,
                        semanticsLabel: l10n.settingsCloseSemantics,
                        onPress: _busy ? null : () => _close(dirty),
                        child: const Icon(LucideIcons.x),
                      ),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            l10n.settingsTitle,
                            style: theme.typography.body.xs.copyWith(
                              fontWeight: FontWeight.w700,
                              letterSpacing: 2,
                            ),
                          ),
                          const SizedBox(height: 4),
                          Text(
                            l10n.settingsSubtitle,
                            style: theme.typography.body.sm.copyWith(
                              color: theme.colors.mutedForeground,
                            ),
                          ),
                        ],
                      ),
                    ),
                    if (dirty)
                      AppButton(
                        size: .sm,
                        mainAxisSize: .min,
                        onPress: _busy ? null : () => _save(draft),
                        child: Text(
                          _busy ? l10n.settingsSaving : l10n.settingsSave,
                        ),
                      ),
                  ],
                ),
              ),
            ),
            Expanded(
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  SettingsSidebar(
                    active: _section,
                    onSelect: (section) => setState(() {
                      if (section != _section) {
                        // One-shot deep-link glow; don't kill on re-tap.
                        _highlightPreviewBus = false;
                      }
                      _section = section;
                    }),
                  ),
                  Expanded(
                    child: SingleChildScrollView(
                      // Full-bleed width: the panel owns the header inset and
                      // lets its cards run to the pane edges.
                      padding: const EdgeInsets.symmetric(vertical: 24),
                      child: _SettingsSectionPanel(
                        section: _section,
                        draft: draft,
                        highlightPreviewBus: _highlightPreviewBus,
                        previewBusCardKey: _previewBusCardKey,
                        onChanged: (next) => setState(() => _draft = next),
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ],
        );
      },
    );
  }

  Future<bool> _save(AppSettings draft) async {
    if (!mounted || _busy) {
      return false;
    }
    setState(() => _busy = true);
    try {
      final previousDeckCount = _baseline?.numDecks;
      if (previousDeckCount != null && previousDeckCount != draft.numDecks) {
        final l10n = AppLocalizations.of(context)!;
        final confirmed = await showMixarConfirm<bool>(
          context: context,
          title: l10n.settingsDeckCountRestartTitle,
          body: l10n.settingsDeckCountRestartBody,
          actions: [
            MixarDialogAction(
              label: l10n.settingsCancel,
              value: false,
              variant: MixarButtonVariant.outline,
            ),
            MixarDialogAction(
              label: l10n.settingsDeckCountRestartConfirm,
              value: true,
            ),
          ],
        );
        if (!mounted || confirmed != true) {
          return false;
        }
      }
      final result = await saveAppSettings(ref, draft);
      if (!mounted) {
        return false;
      }
      if (result.trustedControllersChanged) {
        ref.invalidate(controllerTransportProvider);
        ref.invalidate(controllerMappingsProvider);
        ref.invalidate(controllerDevicesProvider);
      }
      setState(() {
        _draft = result.saved;
        _baseline = result.saved;
      });
      final l10n = AppLocalizations.of(context)!;
      if (result.applyError != null) {
        showMixarToast(
          context: context,
          title: Text(l10n.settingsSavedNotAppliedToast),
          description: Text(result.applyError!),
          variant: MixarToastVariant.destructive,
        );
      } else {
        showMixarToast(context: context, title: Text(l10n.settingsSavedToast));
      }
      return result.applyError == null;
    } catch (e) {
      if (mounted) {
        final l10n = AppLocalizations.of(context)!;
        showMixarToast(
          context: context,
          title: Text(l10n.settingsSaveFailedToast),
          description: Text('$e'),
          variant: MixarToastVariant.destructive,
        );
      }
      return false;
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  Future<void> _close(bool dirty) async {
    if (!mounted || _busy) {
      return;
    }
    if (!dirty) {
      widget.onClose?.call();
      return;
    }
    final l10n = AppLocalizations.of(context)!;
    final choice = await showMixarConfirm<_CloseChoice>(
      context: context,
      title: l10n.settingsUnsavedTitle,
      body: l10n.settingsUnsavedBody,
      actions: [
        MixarDialogAction(
          label: l10n.settingsCancel,
          value: _CloseChoice.cancel,
          variant: MixarButtonVariant.outline,
        ),
        MixarDialogAction(
          label: l10n.settingsDiscard,
          value: _CloseChoice.discard,
          variant: MixarButtonVariant.destructive,
        ),
        MixarDialogAction(label: l10n.settingsSave, value: _CloseChoice.save),
      ],
    );
    if (!mounted) {
      return;
    }
    switch (choice) {
      case null:
      case _CloseChoice.cancel:
        return;
      case _CloseChoice.discard:
        widget.onClose?.call();
        return;
      case _CloseChoice.save:
        final draft = _draft;
        if (draft == null) {
          return;
        }
        final savedAndApplied = await _save(draft);
        if (mounted && savedAndApplied) {
          widget.onClose?.call();
        }
    }
  }
}

enum _CloseChoice { save, discard, cancel }

class _SettingsSectionPanel extends StatelessWidget {
  const new({
    required this.section,
    required this.draft,
    required this.onChanged,
    required this.previewBusCardKey,
    this.highlightPreviewBus = false,
  });

  final SettingsSection section;
  final AppSettings draft;
  final ValueChanged<AppSettings> onChanged;
  final GlobalKey previewBusCardKey;
  final bool highlightPreviewBus;

  @override
  Widget build(BuildContext context) {
    return switch (section) {
      SettingsSection.audio => SettingsAudioPanel(
        draft: draft,
        onChanged: onChanged,
        highlightPreviewBus: highlightPreviewBus,
        previewBusCardKey: previewBusCardKey,
      ),
      SettingsSection.mixer => SettingsMixerPanel(
        draft: draft,
        onChanged: onChanged,
      ),
      SettingsSection.waveform => SettingsWaveformPanel(
        draft: draft,
        onChanged: onChanged,
      ),
      SettingsSection.deck => SettingsDeckPanel(
        draft: draft,
        onChanged: onChanged,
      ),
      SettingsSection.ui => SettingsUiPanel(draft: draft, onChanged: onChanged),
      SettingsSection.library => SettingsLibraryPanel(
        draft: draft,
        onChanged: onChanged,
      ),
      SettingsSection.storage => const SettingsStoragePanel(),
      SettingsSection.session => SettingsSessionPanel(
        draft: draft,
        onChanged: onChanged,
      ),
      SettingsSection.privacy => SettingsPrivacyPanel(
        draft: draft,
        onChanged: onChanged,
      ),
      SettingsSection.controllers => SettingsControllersPanel(
        draft: draft,
        onChanged: onChanged,
      ),
    };
  }
}
