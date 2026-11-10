import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/models/settings_section.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/widgets/shell/m_tappable.dart';
import 'package:gui_flutter/widgets/shell/mixar_input.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// The full-bleed filter band pinned above the settings nav list.
///
/// Tests read its decoration to check the idle/focused wash without depending
/// on where the field sits in the column.
const Key kSettingsSearchBandKey = ValueKey('settings-search-band');

class SettingsSidebar extends StatefulWidget {
  const new({
    required this.active,
    required this.onSelect,
    required this.onQueryChanged,
    super.key,
  });

  final SettingsSection active;
  final ValueChanged<SettingsSection> onSelect;

  /// Reports the live search text so the active panel can filter its cards.
  final ValueChanged<String> onQueryChanged;

  @override
  State<SettingsSidebar> createState() => _SettingsSidebarState();
}

class _SettingsSidebarState extends State<SettingsSidebar> {
  final _searchController = TextEditingController();
  final _searchFocus = FocusNode();

  @override
  void initState() {
    super.initState();
    _searchFocus.addListener(_onSearchFocusChanged);
  }

  @override
  void dispose() {
    _searchFocus.removeListener(_onSearchFocusChanged);
    _searchController.dispose();
    _searchFocus.dispose();
    super.dispose();
  }

  /// The band's wash and hairline follow keyboard focus.
  void _onSearchFocusChanged() => setState(() {});

  void _select(SettingsSection section) {
    _searchFocus.unfocus();
    widget.onSelect(section);
  }

  List<SettingsSection> _filteredSections(AppLocalizations l10n) =>
      matchingSettingsSections(_searchController.text, l10n);

  void _onSearchChanged(String query) {
    setState(() {});
    widget.onQueryChanged(query);
    _ensureSelectionMatches();
  }

  void _ensureSelectionMatches() {
    final l10n = AppLocalizations.of(context)!;
    final matches = _filteredSections(l10n);
    if (matches.isEmpty || matches.contains(widget.active)) {
      return;
    }
    widget.onSelect(matches.first);
  }

  void _clearSearch() {
    _searchController.clear();
    setState(() {});
    widget.onQueryChanged('');
    _searchFocus.requestFocus();
  }

  void _submitSearch(List<SettingsSection> matches) {
    if (matches.isEmpty) {
      return;
    }
    _select(matches.first);
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final colors = theme.colors;
    final searchFocused = _searchFocus.hasFocus;
    final l10n = AppLocalizations.of(context)!;
    final matches = _filteredSections(l10n);
    return DecoratedBox(
      // Foreground: otherwise a selected / hovered nav item's fill covers the
      // divider along the sidebar's right edge.
      position: DecorationPosition.foreground,
      decoration: BoxDecoration(
        border: Border(right: BorderSide(color: theme.colors.border)),
      ),
      child: SizedBox(
        width: 176,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            // Pinned above the scrolling list: the filter is the first thing in
            // the menu and stays reachable however far the sections scroll.
            //
            // Full-bleed and square, like the nav rows below it: rounded field
            // chrome would read as a form control dropped into the column. The
            // wash lifts one solid step (background → secondary → card) and the
            // hairline turns accent while the field owns the keyboard, which is
            // the only border this field needs.
            DecoratedBox(
              key: kSettingsSearchBandKey,
              decoration: BoxDecoration(
                color: searchFocused ? colors.card : colors.secondary,
                border: Border(
                  bottom: BorderSide(
                    color: searchFocused ? colors.primary : colors.border,
                    width: theme.style.borderWidth,
                  ),
                ),
              ),
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: 12),
                child: MixarInput(
                  controller: _searchController,
                  focusNode: _searchFocus,
                  hint: l10n.settingsSearchHint,
                  borderless: true,
                  style: theme.typography.body.sm,
                  padding: const EdgeInsets.symmetric(vertical: 13),
                  textInputAction: TextInputAction.search,
                  leading: Icon(
                    LucideIcons.search,
                    size: 16,
                    color: searchFocused
                        ? colors.foreground
                        : colors.mutedForeground,
                  ),
                  trailing: _searchController.text.isEmpty
                      ? null
                      : MTappable.child(
                          onPress: _clearSearch,
                          semanticsLabel: l10n.commonClear,
                          child: Padding(
                            padding: const EdgeInsets.all(4),
                            child: Icon(
                              LucideIcons.x,
                              size: 14,
                              color: colors.mutedForeground,
                            ),
                          ),
                        ),
                  onChanged: _onSearchChanged,
                  onSubmitted: (_) => _submitSearch(matches),
                ),
              ),
            ),
            Expanded(
              child: SingleChildScrollView(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    if (matches.isEmpty)
                      Padding(
                        padding: const EdgeInsets.fromLTRB(12, 12, 12, 12),
                        child: Text(
                          l10n.settingsSearchNoResults,
                          style: theme.typography.body.sm.copyWith(
                            color: theme.colors.mutedForeground,
                          ),
                        ),
                      )
                    else
                      for (final section in matches)
                        _SettingsNavItem(
                          label: section.label(l10n),
                          icon: _iconFor(section),
                          selected: section == widget.active,
                          onPress: () => _select(section),
                        ),
                  ],
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _SettingsNavItem extends StatelessWidget {
  const new({
    required this.label,
    required this.icon,
    required this.selected,
    required this.onPress,
  });

  final String label;
  final IconData icon;
  final bool selected;
  final VoidCallback onPress;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final colors = theme.colors;

    return MTappable(
      selected: selected,
      semanticsLabel: label,
      onPress: onPress,
      builder: (context, state) {
        final fill = selected
            ? colors.selection
            : state.hovered
            ? colors.card
            : null;
        return DecoratedBox(
          decoration: BoxDecoration(
            color: fill,
            border: Border(
              left: BorderSide(
                width: 2,
                color: selected ? colors.primary : const Color(0x00000000),
              ),
            ),
          ),
          child: Padding(
            // A roomier row than the default chrome: the nav is a long list
            // read top to bottom, so taller rows and a wider icon gap keep the
            // labels from crowding each other and the pane's edge.
            padding: const EdgeInsets.fromLTRB(12, 12, 8, 12),
            child: Row(
              children: [
                Icon(
                  icon,
                  size: 16,
                  // Selected rows carry the brand accent on the icon only; the
                  // label stays selection-foreground so it reads as chrome.
                  color: selected ? colors.primary : colors.mutedForeground,
                ),
                const SizedBox(width: 10),
                Expanded(
                  child: Text(
                    label,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: theme.typography.body.sm.copyWith(
                      color: selected
                          ? colors.selectionForeground
                          : colors.foreground,
                      fontWeight: FontWeight.w500,
                    ),
                  ),
                ),
              ],
            ),
          ),
        );
      },
    );
  }
}

IconData _iconFor(SettingsSection section) {
  return switch (section) {
    SettingsSection.audio => LucideIcons.volume2,
    SettingsSection.mixer => LucideIcons.slidersHorizontal,
    SettingsSection.curves => LucideIcons.spline,
    SettingsSection.waveform => LucideIcons.audioWaveform,
    SettingsSection.deck => LucideIcons.disc3,
    SettingsSection.ui => LucideIcons.panelTop,
    SettingsSection.layout => LucideIcons.layoutDashboard,
    SettingsSection.library => LucideIcons.library,
    SettingsSection.integrations => LucideIcons.plug,
    SettingsSection.storage => LucideIcons.hardDrive,
    SettingsSection.session => LucideIcons.history,
    SettingsSection.privacy => LucideIcons.shield,
    SettingsSection.controllers => LucideIcons.gamepad2,
  };
}
