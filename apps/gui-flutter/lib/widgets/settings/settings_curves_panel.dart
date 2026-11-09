import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/models/curve_editor.dart';
import 'package:gui_flutter/core/theme/mixar_button_style.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/src/rust/api/event_mirrors.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/curves/curve_dialogs.dart';
import 'package:gui_flutter/widgets/curves/curve_group_editor.dart';
import 'package:gui_flutter/widgets/curves/curve_import_export_dialog.dart';
import 'package:gui_flutter/widgets/curves/curve_set_editor.dart';
import 'package:gui_flutter/widgets/settings/settings_section_body.dart';
import 'package:gui_flutter/widgets/settings/settings_section_card.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/m_tappable.dart';
import 'package:gui_flutter/widgets/shell/mixar_dialog.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:gui_flutter/widgets/shell/mixar_toast.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Custom curve library: point editing, set morphing, and VirtualDJ I/O.
class SettingsCurvesPanel extends StatefulWidget {
  const new({
    required this.draft,
    required this.onChanged,
    this.importer,
    this.exporter,
    super.key,
  });

  final AppSettings draft;
  final ValueChanged<AppSettings> onChanged;

  /// Bridge hooks; tests inject fakes so they never start the Rust runtime.
  final VirtualDjCurveImporter? importer;
  final VirtualDjCurveExporter? exporter;

  @override
  State<SettingsCurvesPanel> createState() => _SettingsCurvesPanelState();
}

class _SettingsCurvesPanelState extends State<SettingsCurvesPanel> {
  String? _selectedGroupId;
  String? _selectedSetId;

  CurveLibrarySpec get _library => widget.draft.curveLibrary;

  CurveGroupSpec? get _selectedGroup {
    final groups = _library.groups;
    if (groups.isEmpty) {
      return null;
    }
    for (final group in groups) {
      if (group.id == _selectedGroupId) {
        return group;
      }
    }
    return groups.first;
  }

  CurveSetSpec? get _selectedSet {
    final sets = _library.sets;
    if (sets.isEmpty) {
      return null;
    }
    for (final set in sets) {
      if (set.id == _selectedSetId) {
        return set;
      }
    }
    return sets.first;
  }

  void _applyLibrary(CurveLibrarySpec library) {
    widget.onChanged(copyAppSettings(widget.draft, curveLibrary: library));
  }

  void _selectGroup(String id) => setState(() => _selectedGroupId = id);

  void _selectSet(String id) => setState(() => _selectedSetId = id);

  Future<void> _createGroup() async {
    final group = await showCreateCurveGroupDialog(context, library: _library);
    if (!mounted || group == null) {
      return;
    }
    _applyLibrary(
      copyCurveLibrary(_library, groups: [..._library.groups, group]),
    );
    _selectGroup(group.id);
  }

  Future<void> _renameGroup(CurveGroupSpec group) async {
    final name = await showRenameCurveGroupDialog(context, group: group);
    if (!mounted || name == null) {
      return;
    }
    _applyLibrary(
      copyCurveLibrary(
        _library,
        groups: [
          for (final candidate in _library.groups)
            if (candidate.id == group.id)
              copyCurveGroup(candidate, displayName: name)
            else
              candidate,
        ],
      ),
    );
  }

  Future<void> _deleteGroup(CurveGroupSpec group) async {
    final l10n = AppLocalizations.of(context)!;
    final usedBy = _library.sets
        .where((set) => set.members.any((member) => member.id == group.id))
        .toList();
    if (usedBy.isNotEmpty) {
      showMixarToast(
        context: context,
        title: Text(l10n.curvesInUseTitle),
        description: Text(l10n.curvesInUseBody(_groupName(group))),
      );
      return;
    }
    if (widget.draft.defaultCrossfaderCurve == group.id) {
      showMixarToast(
        context: context,
        title: Text(l10n.curvesDefaultInUseTitle),
        description: Text(l10n.curvesDefaultInUseBody(_groupName(group))),
      );
      return;
    }
    final confirmed = await showMixarConfirm<bool>(
      context: context,
      title: l10n.curvesDeleteConfirmTitle,
      body: l10n.curvesDeleteConfirmBody(_groupName(group)),
      actions: [
        MixarDialogAction(
          label: l10n.curvesCancel,
          value: false,
          variant: MixarButtonVariant.outline,
        ),
        MixarDialogAction(
          label: l10n.curvesDelete,
          value: true,
          variant: MixarButtonVariant.destructive,
        ),
      ],
    );
    if (!mounted || confirmed != true) {
      return;
    }
    _applyLibrary(
      copyCurveLibrary(
        _library,
        groups: [
          for (final candidate in _library.groups)
            if (candidate.id != group.id) candidate,
        ],
      ),
    );
    if (_selectedGroupId == group.id) {
      setState(() => _selectedGroupId = null);
    }
  }

  Future<void> _createSet() async {
    final set = await showCreateCurveSetDialog(context, library: _library);
    if (!mounted || set == null) {
      return;
    }
    _applyLibrary(copyCurveLibrary(_library, sets: [..._library.sets, set]));
    _selectSet(set.id);
  }

  Future<void> _renameSet(CurveSetSpec set) async {
    final name = await showRenameCurveSetDialog(context, set: set);
    if (!mounted || name == null) {
      return;
    }
    _applyLibrary(
      copyCurveLibrary(
        _library,
        sets: [
          for (final candidate in _library.sets)
            if (candidate.id == set.id)
              copyCurveSet(candidate, displayName: name)
            else
              candidate,
        ],
      ),
    );
  }

  Future<void> _deleteSet(CurveSetSpec set) async {
    final l10n = AppLocalizations.of(context)!;
    if (widget.draft.defaultCrossfaderCurve == set.id) {
      showMixarToast(
        context: context,
        title: Text(l10n.curvesDefaultInUseTitle),
        description: Text(l10n.curvesDefaultInUseBody(_setName(set))),
      );
      return;
    }
    final confirmed = await showMixarConfirm<bool>(
      context: context,
      title: l10n.curvesDeleteSetConfirmTitle,
      body: l10n.curvesDeleteConfirmBody(_setName(set)),
      actions: [
        MixarDialogAction(
          label: l10n.curvesCancel,
          value: false,
          variant: MixarButtonVariant.outline,
        ),
        MixarDialogAction(
          label: l10n.curvesDelete,
          value: true,
          variant: MixarButtonVariant.destructive,
        ),
      ],
    );
    if (!mounted || confirmed != true) {
      return;
    }
    _applyLibrary(
      copyCurveLibrary(
        _library,
        sets: [
          for (final candidate in _library.sets)
            if (candidate.id != set.id) candidate,
        ],
      ),
    );
    if (_selectedSetId == set.id) {
      setState(() => _selectedSetId = null);
    }
  }

  Future<void> _importVirtualDj() async {
    final group = await showImportVirtualDjCurveDialog(
      context,
      importer: widget.importer,
    );
    if (!mounted || group == null) {
      return;
    }
    final id = _uniqueId(slugifyCurveId(group.displayName), _library);
    final imported = copyCurveGroup(group, id: id);
    _applyLibrary(
      copyCurveLibrary(_library, groups: [..._library.groups, imported]),
    );
    _selectGroup(imported.id);
    final l10n = AppLocalizations.of(context)!;
    showMixarToast(
      context: context,
      title: Text(l10n.curvesImported(_groupName(imported))),
    );
  }

  Future<void> _export(CurveGroupSpec group) async {
    await showExportVirtualDjCurveDialog(
      context,
      group: group,
      exporter: widget.exporter,
    );
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final l10n = AppLocalizations.of(context)!;
    final selectedGroup = _selectedGroup;
    final selectedSet = _selectedSet;
    final libraryError = curveLibraryValidationError(_library);

    return SettingsSectionBody(
      title: l10n.settingsSectionCurves,
      description: l10n.settingsCurvesDescription,
      before: libraryError == null
          ? null
          : Text(
              libraryError,
              style: theme.typography.body.xs.copyWith(
                color: theme.colors.destructive,
              ),
            ),
      cards: [
        SettingsSectionCard(
          title: l10n.curvesCustomTitle,
          subtitle: l10n.curvesCustomDescription,
          trailing: AppButton(
            size: .sm,
            mainAxisSize: .min,
            onPress: _createGroup,
            child: Text(l10n.curvesNew),
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            spacing: 12,
            children: [
              if (_library.groups.isEmpty)
                Text(
                  l10n.curvesEmpty,
                  style: theme.typography.body.sm.copyWith(
                    color: theme.colors.mutedForeground,
                  ),
                )
              else
                for (final group in _library.groups)
                  _LibraryRow(
                    title: _groupName(group),
                    subtitle: group.id,
                    badge: group.curves.length > 1
                        ? l10n.curvesAb
                        : l10n.curvesScalar,
                    selected: selectedGroup?.id == group.id,
                    onPress: () => _selectGroup(group.id),
                    onRename: () => _renameGroup(group),
                    onDelete: () => _deleteGroup(group),
                  ),
              if (selectedGroup != null) ...[
                const SizedBox(height: 4),
                CurveGroupEditor(
                  key: ValueKey('curve-group-${selectedGroup.id}'),
                  group: selectedGroup,
                  onChanged: (group) => _applyLibrary(
                    copyCurveLibrary(
                      _library,
                      groups: [
                        for (final candidate in _library.groups)
                          if (candidate.id == group.id) group else candidate,
                      ],
                    ),
                  ),
                ),
              ] else if (_library.groups.isNotEmpty)
                Text(
                  l10n.curvesSelectToEdit,
                  style: theme.typography.body.sm.copyWith(
                    color: theme.colors.mutedForeground,
                  ),
                ),
            ],
          ),
        ),
        SettingsSectionCard(
          title: l10n.curvesSetsTitle,
          subtitle: l10n.curvesSetsDescription,
          trailing: AppButton(
            size: .sm,
            mainAxisSize: .min,
            onPress: _createSet,
            child: Text(l10n.curvesNewSet),
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            spacing: 12,
            children: [
              if (_library.sets.isEmpty)
                Text(
                  l10n.curvesEmptySets,
                  style: theme.typography.body.sm.copyWith(
                    color: theme.colors.mutedForeground,
                  ),
                )
              else
                for (final set in _library.sets)
                  _LibraryRow(
                    title: _setName(set),
                    subtitle: l10n.curvesMemberCount(set.members.length),
                    badge: l10n.curvesAb,
                    selected: selectedSet?.id == set.id,
                    onPress: () => _selectSet(set.id),
                    onRename: () => _renameSet(set),
                    onDelete: () => _deleteSet(set),
                  ),
              if (selectedSet != null) ...[
                const SizedBox(height: 4),
                CurveSetEditor(
                  key: ValueKey('curve-set-${selectedSet.id}'),
                  set: selectedSet,
                  library: _library,
                  onChanged: (set) => _applyLibrary(
                    copyCurveLibrary(
                      _library,
                      sets: [
                        for (final candidate in _library.sets)
                          if (candidate.id == set.id) set else candidate,
                      ],
                    ),
                  ),
                ),
              ] else if (_library.sets.isNotEmpty)
                Text(
                  l10n.curvesSelectSetToEdit,
                  style: theme.typography.body.sm.copyWith(
                    color: theme.colors.mutedForeground,
                  ),
                ),
            ],
          ),
        ),
        SettingsSectionCard(
          title: l10n.curvesImportExportTitle,
          subtitle: l10n.curvesImportExportDescription,
          child: Wrap(
            spacing: 12,
            runSpacing: 8,
            children: [
              AppButton(
                size: .sm,
                mainAxisSize: .min,
                variant: .outline,
                onPress: _importVirtualDj,
                child: Text(l10n.curvesImport),
              ),
              AppButton(
                size: .sm,
                mainAxisSize: .min,
                variant: .outline,
                onPress: selectedGroup == null
                    ? null
                    : () => _export(selectedGroup),
                child: Text(l10n.curvesExport),
              ),
            ],
          ),
        ),
      ],
    );
  }

  String _groupName(CurveGroupSpec group) {
    return group.displayName.isEmpty ? group.id : group.displayName;
  }

  String _setName(CurveSetSpec set) {
    return set.displayName.isEmpty ? set.id : set.displayName;
  }

  String _uniqueId(String base, CurveLibrarySpec library) {
    final trimmed = base.length > 60 ? base.substring(0, 60) : base;
    var candidate = trimmed;
    var suffix = 2;
    while (kReservedCurvePresetIds.contains(candidate) ||
        library.groups.any((group) => group.id == candidate) ||
        library.sets.any((set) => set.id == candidate)) {
      candidate = '${trimmed}_$suffix';
      suffix += 1;
    }
    return candidate;
  }
}

class _LibraryRow extends StatelessWidget {
  const new({
    required this.title,
    required this.subtitle,
    required this.badge,
    required this.selected,
    required this.onPress,
    required this.onRename,
    required this.onDelete,
  });

  final String title;
  final String subtitle;
  final String badge;
  final bool selected;
  final VoidCallback onPress;
  final VoidCallback onRename;
  final VoidCallback onDelete;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final l10n = AppLocalizations.of(context)!;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: selected ? theme.colors.selection : null,
        border: Border(bottom: BorderSide(color: theme.colors.border)),
      ),
      child: Row(
        children: [
          Expanded(
            child: MTappable(
              selected: selected,
              semanticsLabel: '$title, $subtitle',
              onPress: onPress,
              builder: (context, state) => Padding(
                padding: const EdgeInsets.symmetric(vertical: 8),
                child: Row(
                  children: [
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            title,
                            style: theme.typography.body.sm.copyWith(
                              fontWeight: FontWeight.w600,
                              color: selected
                                  ? theme.colors.selectionForeground
                                  : theme.colors.foreground,
                            ),
                          ),
                          Text(
                            subtitle,
                            style: theme.typography.body.xs.copyWith(
                              color: selected
                                  ? theme.colors.selectionForeground
                                  : theme.colors.mutedForeground,
                            ),
                          ),
                        ],
                      ),
                    ),
                    Text(
                      badge,
                      style: theme.typography.body.xs.copyWith(
                        color: theme.colors.mutedForeground,
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ),
          AppButton.icon(
            size: .sm,
            variant: .ghost,
            semanticsLabel: l10n.curvesRename,
            onPress: onRename,
            child: const Icon(LucideIcons.pencil, size: 16),
          ),
          AppButton.icon(
            size: .sm,
            variant: .ghost,
            semanticsLabel: l10n.curvesDelete,
            onPress: onDelete,
            child: const Icon(LucideIcons.trash2, size: 16),
          ),
        ],
      ),
    );
  }
}
