import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/models/settings_section.dart';
import 'package:gui_flutter/core/theme/mixar_button_style.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/providers/settings/settings_leave_guard.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/providers/shell/controller_providers.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_analysis_panel.dart';
import 'package:gui_flutter/widgets/settings/settings_audio_panel.dart';
import 'package:gui_flutter/widgets/settings/settings_controllers_panel.dart';
import 'package:gui_flutter/widgets/settings/settings_curves_panel.dart';
import 'package:gui_flutter/widgets/settings/settings_deck_panel.dart';
import 'package:gui_flutter/widgets/settings/settings_integrations_panel.dart';
import 'package:gui_flutter/widgets/settings/settings_layout_panel.dart';
import 'package:gui_flutter/widgets/settings/settings_library_panel.dart';
import 'package:gui_flutter/widgets/settings/settings_mixer_panel.dart';
import 'package:gui_flutter/widgets/settings/settings_privacy_panel.dart';
import 'package:gui_flutter/widgets/settings/settings_search_scope.dart';
import 'package:gui_flutter/widgets/settings/settings_session_panel.dart';
import 'package:gui_flutter/widgets/settings/settings_sidebar.dart';
import 'package:gui_flutter/widgets/settings/settings_storage_panel.dart';
import 'package:gui_flutter/widgets/settings/settings_ui_panel.dart';
import 'package:gui_flutter/widgets/settings/settings_waveform_panel.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/mixar_dialog.dart';
import 'package:gui_flutter/widgets/shell/mixar_input.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:gui_flutter/widgets/shell/mixar_toast.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Scroll view holding the active section's panel.
///
/// The sidebar scrolls on its own, so tests that measure or drive the page
/// content target this key instead of the first `SingleChildScrollView`.
const kSettingsContentScrollKey = ValueKey('settings-content-scroll');

/// Height of the Settings page's own header band (glyph, title, search, save).
const double kSettingsHeaderHeight = 60;

/// Slack between the header's right cluster members.
const double _kSettingsHeaderGap = 8;

/// How long the toggle takes to slide between its two seats, and how the Save
/// slot opens under it.
const Duration _kSettingsHeaderSlide = Duration(milliseconds: 200);
const Curve _kSettingsHeaderCurve = Curves.easeOutCubic;

class SettingsPage extends ConsumerStatefulWidget {
  const new({
    this.initialSection = SettingsSection.ui,
    this.scrollToPreviewBus = false,
    super.key,
  }) : assert(
         !scrollToPreviewBus || initialSection == SettingsSection.audio,
         'scrollToPreviewBus requires initialSection.audio',
       );

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

  /// Header search: the button expands the title row into a field.
  var _searchOpen = false;
  var _searchQuery = '';
  final _searchController = TextEditingController();
  final _searchFocus = FocusNode();

  /// Section the user was on when the search first hid it. Clearing puts them
  /// back there instead of stranding them on a section the search picked.
  SettingsSection? _sectionBeforeSearch;

  /// Captured on mount: `ref` is unsafe to touch from [dispose].
  SettingsLeaveGuard? _leaveGuard;

  bool _isDirty(AppSettings draft, AppSettings baseline) =>
      appSettingsDirty(draft, baseline);

  /// Adopt settings saved outside this page, such as the controller audio
  /// prompt switching the output buses.
  ///
  /// A clean draft is replaced outright. A dirty draft keeps its edits, but the
  /// audio-routing fields the user did not touch follow the new saved value, so
  /// the Audio panel never shows a device Mixar is no longer using.
  void _adoptSavedSettings(AppSettings saved) {
    final draft = _draft;
    final baseline = _baseline;
    if (draft == null || baseline == null) {
      setState(() {
        _draft = saved;
        _baseline = saved;
      });
      return;
    }
    if (saved == baseline) {
      return;
    }
    setState(() {
      if (draft == baseline) {
        _draft = saved;
      } else {
        _draft = copyAppSettings(
          draft,
          masterBus: draft.masterBus == baseline.masterBus
              ? saved.masterBus
              : draft.masterBus,
          previewEnabled: draft.previewEnabled == baseline.previewEnabled
              ? saved.previewEnabled
              : draft.previewEnabled,
          previewBus: draft.previewBus == baseline.previewBus
              ? saved.previewBus
              : draft.previewBus,
          controllerAudioOverride:
              draft.controllerAudioOverride == baseline.controllerAudioOverride
              ? saved.controllerAudioOverride
              : draft.controllerAudioOverride,
          trustedControllerDeviceIds:
              listEquals(
                draft.trustedControllerDeviceIds,
                baseline.trustedControllerDeviceIds,
              )
              ? saved.trustedControllerDeviceIds
              : draft.trustedControllerDeviceIds,
        );
      }
      _baseline = saved;
    });
  }

  @override
  void initState() {
    super.initState();
    // Registering the guard writes a provider, which the widget tree forbids
    // while it builds; the first frame is the earliest slot for it.
    final leaveGuard = ref.read(settingsLeaveGuardProvider.notifier);
    _leaveGuard = leaveGuard;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) {
        leaveGuard.set(_confirmLeave);
      }
    });
    if (widget.scrollToPreviewBus) {
      WidgetsBinding.instance.addPostFrameCallback((_) => _tryPreviewScroll());
    }
  }

  @override
  void dispose() {
    // Deregistering writes a provider, which the tree forbids mid-teardown:
    // defer it, and let `clearIf` drop it as a no-op if the container is gone.
    final leaveGuard = _leaveGuard;
    final guard = _confirmLeave;
    if (leaveGuard != null) {
      Future.microtask(() => leaveGuard.clearIf(guard));
    }
    _searchController.dispose();
    _searchFocus.dispose();
    super.dispose();
  }

  void _openSearch() {
    setState(() => _searchOpen = true);
    _searchFocus.requestFocus();
  }

  /// Collapses the header back to the page title and drops the query, so the
  /// nav list and the panel show everything again.
  void _closeSearch() {
    _searchFocus.unfocus();
    _searchController.clear();
    setState(() {
      _searchOpen = false;
      _searchQuery = '';
    });
    _restoreSectionAfterSearch();
  }

  void _onSearchChanged(String query) {
    setState(() => _searchQuery = query);
    if (query.trim().isEmpty) {
      _restoreSectionAfterSearch();
      return;
    }
    _ensureSelectionMatches();
  }

  /// Keeps the open section inside the filtered nav list.
  void _ensureSelectionMatches() {
    final l10n = AppLocalizations.of(context)!;
    final matches = matchingSettingsSections(_searchQuery, l10n);
    if (matches.isEmpty || matches.contains(_section)) {
      return;
    }
    _sectionBeforeSearch ??= _section;
    _selectFromSearch(matches.first);
  }

  /// Returns to the section the search hid, if the query was cleared.
  void _restoreSectionAfterSearch() {
    final restore = _sectionBeforeSearch;
    _sectionBeforeSearch = null;
    if (restore != null) {
      _selectFromSearch(restore);
    }
  }

  /// Search-driven section move (auto-pick, or restore when the query clears).
  ///
  /// Not a user pick: the one-shot deep-link glow stays armed, and landing back
  /// on Audio re-arms its pending scroll in case the first attempt was skipped
  /// while another section owned the pane.
  void _selectFromSearch(SettingsSection section) {
    setState(() => _section = section);
    if (section != SettingsSection.audio ||
        !widget.scrollToPreviewBus ||
        _didSchedulePreviewScroll) {
      return;
    }
    // A previous poll may have spent its attempts while another section owned
    // the pane; restart the budget so this re-arm can actually reach the card.
    _previewScrollAttempts = 0;
    WidgetsBinding.instance.addPostFrameCallback((_) => _tryPreviewScroll());
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
    final benchmark = ref.watch(realtimeStemsBenchmarkProvider);
    ref.listen(appSettingsProvider, (previous, next) {
      final saved = next.asData?.value;
      if (saved != null) {
        _adoptSavedSettings(saved);
      }
    });

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
        final searchQuery = _searchQuery.trim();
        final noSectionMatches = matchingSettingsSections(
          _searchQuery,
          l10n,
        ).isEmpty;
        // The band grows with the text scale: the title pair is the tallest
        // thing in it, and a fixed square would clip it on a scaled desktop.
        // The search square is one band tall, so it grows with it.
        final headerHeight = MediaQuery.textScalerOf(context)
            .scale(kSettingsHeaderHeight);
        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            DecoratedBox(
              decoration: BoxDecoration(
                color: theme.colors.card,
                border: Border(bottom: BorderSide(color: theme.colors.border)),
              ),
              // A fixed height, not IntrinsicHeight: the row holds a text
              // field in its search state, and an intrinsic pass over a field
              // and a square cell fights over the same dimension.
              child: SizedBox(
                height: headerHeight,
                child: LayoutBuilder(
                  builder: (context, constraints) {
                    // The toggle travels between the corner cell and the right
                    // end, so both ends are plain geometry: no key lookups and
                    // nothing to measure mid-animation.
                    final saveLabel = _busy
                        ? l10n.settingsSaving
                        : l10n.settingsSaveChanges;
                    final saveWidth = dirty
                        ? _saveSlotWidth(context, theme, saveLabel)
                        : 0.0;
                    // The right edge is a square in both states — the search
                    // toggle, then the clear button — with the Save slot
                    // opening to its left. Open, the toggle becomes the corner
                    // cell instead.
                    final rightCluster =
                        headerHeight +
                        (saveWidth == 0
                            ? 0.0
                            : saveWidth + 2 * _kSettingsHeaderGap);
                    final toggleLeft = _searchOpen
                        ? 0.0
                        : constraints.maxWidth - headerHeight;
                    return Stack(
                      children: [
                        Row(
                          children: [
                            // Square corner cell. Closed, it names the page;
                            // open, the search toggle covers it.
                            SizedBox(
                              width: headerHeight,
                              child: Center(
                                child: AnimatedOpacity(
                                  duration: _kSettingsHeaderSlide,
                                  opacity: _searchOpen ? 0 : 1,
                                  child: Icon(
                                    LucideIcons.settings,
                                    size: 16,
                                    color: theme.colors.mutedForeground,
                                  ),
                                ),
                              ),
                            ),
                            Expanded(
                              child: Padding(
                                padding: const EdgeInsets.only(left: 4),
                                child: _searchOpen
                                    ? Align(
                                        alignment: Alignment.centerLeft,
                                        child: TapRegion(
                                          // A press outside closes an empty
                                          // bar only: with a query in it the
                                          // search is still doing work, so the
                                          // clear button is the way out. The
                                          // press itself reaches what it hit.
                                          onTapOutside: (_) {
                                            if (_searchQuery.trim().isEmpty) {
                                              _closeSearch();
                                            }
                                          },
                                          child: MixarInput(
                                            controller: _searchController,
                                            focusNode: _searchFocus,
                                            hint: l10n.settingsSearchHint,
                                            borderless: true,
                                            style: theme.typography.body.sm,
                                            padding: const EdgeInsets.symmetric(
                                              vertical: 8,
                                            ),
                                            textInputAction:
                                                TextInputAction.search,
                                            onChanged: _onSearchChanged,
                                            onSubmitted: (_) =>
                                                _ensureSelectionMatches(),
                                          ),
                                        ),
                                      )
                                    : Column(
                                        mainAxisAlignment:
                                            MainAxisAlignment.center,
                                        crossAxisAlignment:
                                            CrossAxisAlignment.start,
                                        children: [
                                          // One line each: the band is a fixed
                                          // height, so a wrapped subtitle would
                                          // overflow it. Narrow windows and big
                                          // text scales ellipsize instead.
                                          Text(
                                            l10n.settingsTitle,
                                            maxLines: 1,
                                            overflow: TextOverflow.ellipsis,
                                            style: theme.typography.body.xs
                                                .copyWith(
                                                  fontWeight: FontWeight.w700,
                                                  letterSpacing: 2,
                                                ),
                                          ),
                                          const SizedBox(height: 4),
                                          Text(
                                            l10n.settingsSubtitle,
                                            maxLines: 1,
                                            overflow: TextOverflow.ellipsis,
                                            style: theme.typography.body.sm
                                                .copyWith(
                                                  color: theme
                                                      .colors
                                                      .mutedForeground,
                                                ),
                                          ),
                                        ],
                                      ),
                              ),
                            ),
                            // Reserved so the title and the field never reflow
                            // under the controls; the slots open with the
                            // toggle's own animation.
                            AnimatedContainer(
                              duration: _kSettingsHeaderSlide,
                              curve: _kSettingsHeaderCurve,
                              width: rightCluster,
                            ),
                          ],
                        ),
                        AnimatedPositioned(
                          duration: _kSettingsHeaderSlide,
                          curve: _kSettingsHeaderCurve,
                          left: toggleLeft,
                          top: 0,
                          width: headerHeight,
                          height: headerHeight,
                          child: _searchOpen
                              // Once it arrives it is a marker, not a control:
                              // the header is the search bar, and a press
                              // anywhere outside closes it.
                              ? const IgnorePointer(
                                  child: Center(
                                    child: Icon(LucideIcons.search, size: 16),
                                  ),
                                )
                              : AppButton.icon(
                                  variant: .ghost,
                                  size: .sm,
                                  semanticsLabel: l10n.settingsSearchHint,
                                  onPress: _openSearch,
                                  child: const Icon(LucideIcons.search),
                                ),
                        ),
                        if (_searchOpen)
                          Positioned(
                            right: 0,
                            top: 0,
                            width: headerHeight,
                            height: headerHeight,
                            child: AppButton.icon(
                              variant: .ghost,
                              size: .sm,
                              semanticsLabel: l10n.settingsSearchClose,
                              onPress: _closeSearch,
                              child: const Icon(LucideIcons.x),
                            ),
                          ),
                        if (dirty)
                          AnimatedPositioned(
                            duration: _kSettingsHeaderSlide,
                            curve: _kSettingsHeaderCurve,
                            right: headerHeight + _kSettingsHeaderGap,
                            top: 0,
                            bottom: 0,
                            child: Center(
                              child: SizedBox(
                                width: saveWidth,
                                child: AppButton(
                                  size: .sm,
                                  mainAxisSize: .min,
                                  onPress: _busy ? null : () => _save(draft),
                                  child: Text(saveLabel),
                                ),
                              ),
                            ),
                          ),
                      ],
                    );
                  },
                ),
              ),
            ),
            Expanded(
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  SettingsSidebar(
                    active: _section,
                    query: _searchQuery,
                    onSelect: (section) => setState(() {
                      // An explicit pick is where the user wants to be, so it
                      // survives the query clearing.
                      _sectionBeforeSearch = null;
                      if (section != _section) {
                        // One-shot deep-link glow; don't kill on re-tap.
                        _highlightPreviewBus = false;
                      }
                      _section = section;
                    }),
                  ),
                  Expanded(
                    child: SingleChildScrollView(
                      key: kSettingsContentScrollKey,
                      // Full-bleed width: the panel owns the header inset and
                      // lets its cards run to the pane edges.
                      padding: const EdgeInsets.symmetric(vertical: 24),
                      child: noSectionMatches
                          ? Padding(
                              padding: const EdgeInsets.symmetric(
                                horizontal: 16,
                              ),
                              child: Text(
                                l10n.settingsSearchNoResultsFor(searchQuery),
                                style: theme.typography.body.sm.copyWith(
                                  color: theme.colors.mutedForeground,
                                ),
                              ),
                            )
                          : SettingsSearchScope(
                              query: _searchQuery,
                              child: _SettingsSectionPanel(
                                section: _section,
                                draft: draft,
                                baseline: baseline,
                                highlightPreviewBus: _highlightPreviewBus,
                                previewBusCardKey: _previewBusCardKey,
                                realtimeStemsCapability: benchmark.capability,
                                benchmarkRunning: benchmark.running,
                                benchmarkFraction: benchmark.fraction,
                                benchmarkError: benchmark.error,
                                onRunRealtimeStemsBenchmark: () => ref
                                    .read(
                                      realtimeStemsBenchmarkProvider.notifier,
                                    )
                                    .run(),
                                onChanged: (next) =>
                                    setState(() => _draft = next),
                              ),
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
          title: l10n.settingsLayoutDeckCountRestartTitle,
          body: l10n.settingsLayoutDeckCountRestartBody,
          actions: [
            MixarDialogAction(
              label: l10n.settingsCancel,
              value: false,
              variant: MixarButtonVariant.outline,
            ),
            MixarDialogAction(
              label: l10n.settingsLayoutDeckCountRestartConfirm,
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
          variant: MixarToastVariant.warning,
        );
      } else {
        showMixarToast(
          context: context,
          title: Text(l10n.settingsSavedToast),
          variant: MixarToastVariant.success,
        );
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

  /// Width the Save button needs for [label].
  ///
  /// The toggle is seated left of it, so the slot has to be known before the
  /// row lays out — and it changes with the label ("Save changes" /
  /// "Saving…").
  ///
  /// Measured with the style the button paints with (the app density's weight,
  /// a solid line height) *and* the ambient text scaler: a scaled-up label is
  /// wider than the raw font metrics, and the slot clips what it cannot fit.
  double _saveSlotWidth(
    BuildContext context,
    MixarThemeData theme,
    String label,
  ) {
    final style = theme.typography.body.sm.copyWith(
      fontWeight: FontWeight.w500,
      height: 1,
    );
    final painter = TextPainter(
      text: TextSpan(text: label, style: style),
      textDirection: TextDirection.ltr,
      textScaler: MediaQuery.textScalerOf(context),
    )..layout();
    // The sm button's own horizontal content padding, plus a little slack for
    // font-metric rounding differences.
    final width = painter.width + 20 + 6;
    painter.dispose();
    return width;
  }

  /// Whether the shell may leave this page: true when nothing is unsaved, or
  /// when the user discarded or saved the draft.
  Future<bool> _confirmLeave() async {
    if (!mounted) {
      // The page is gone: nothing left to protect. This is also what a guard
      // registered by an earlier mount answers before the next one lands.
      return true;
    }
    if (_busy) {
      return false;
    }
    final draft = _draft;
    final baseline = _baseline;
    if (draft == null || baseline == null || !_isDirty(draft, baseline)) {
      return true;
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
      return false;
    }
    switch (choice) {
      case null:
      case _CloseChoice.cancel:
        return false;
      case _CloseChoice.discard:
        return true;
      case _CloseChoice.save:
        final draft = _draft;
        if (draft == null) {
          return false;
        }
        final savedAndApplied = await _save(draft);
        return mounted && savedAndApplied;
    }
  }
}

enum _CloseChoice { save, discard, cancel }

class _SettingsSectionPanel extends StatelessWidget {
  const new({
    required this.section,
    required this.draft,
    required this.baseline,
    required this.onChanged,
    required this.previewBusCardKey,
    required this.realtimeStemsCapability,
    required this.benchmarkRunning,
    required this.benchmarkFraction,
    required this.benchmarkError,
    required this.onRunRealtimeStemsBenchmark,
    this.highlightPreviewBus = false,
  });

  final SettingsSection section;
  final AppSettings draft;

  /// Saved settings the app is running with, for fields that must not preview
  /// an unsaved edit.
  final AppSettings baseline;

  final ValueChanged<AppSettings> onChanged;
  final GlobalKey previewBusCardKey;
  final RealtimeStemsCapability? realtimeStemsCapability;
  final bool benchmarkRunning;
  final double? benchmarkFraction;
  final String? benchmarkError;
  final VoidCallback onRunRealtimeStemsBenchmark;
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
        capability: realtimeStemsCapability,
        benchmarkRunning: benchmarkRunning,
        benchmarkFraction: benchmarkFraction,
        benchmarkError: benchmarkError,
        onRunBenchmark: onRunRealtimeStemsBenchmark,
      ),
      SettingsSection.curves => SettingsCurvesPanel(
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
      SettingsSection.ui => SettingsUiPanel(
        draft: draft,
        savedAccentPalette: baseline.accentPalette,
        onChanged: onChanged,
      ),
      SettingsSection.layout => SettingsLayoutPanel(
        draft: draft,
        onChanged: onChanged,
      ),
      SettingsSection.library => SettingsLibraryPanel(
        draft: draft,
        onChanged: onChanged,
      ),
      SettingsSection.analysis => SettingsAnalysisPanel(
        draft: draft,
        onChanged: onChanged,
      ),
      SettingsSection.integrations => SettingsIntegrationsPanel(
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
