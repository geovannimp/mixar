import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/models/curve_editor.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/src/rust/api/event_mirrors.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/curves/curve_import_export_dialog.dart';
import 'package:gui_flutter/widgets/settings/settings_field.dart';
import 'package:gui_flutter/widgets/settings/settings_section_card.dart';
import 'package:gui_flutter/widgets/settings/settings_select.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:gui_flutter/widgets/shell/mixar_toast.dart';

/// Integrations card for the VirtualDJ curve adapter.
///
/// The full curve editor stays out of the application for now; this card only
/// moves an imported curve in and out and lets it become the default.
class CurveImportExportCard extends StatefulWidget {
  const new({
    required this.draft,
    required this.onChanged,
    this.importer,
    this.exporter,
    super.key,
  });

  final AppSettings draft;
  final ValueChanged<AppSettings> onChanged;
  final VirtualDjCurveImporter? importer;
  final VirtualDjCurveExporter? exporter;

  @override
  State<CurveImportExportCard> createState() => _CurveImportExportCardState();
}

class _CurveImportExportCardState extends State<CurveImportExportCard> {
  String? _selectedGroupId;

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

  @override
  void initState() {
    super.initState();
    _selectedGroupId = _defaultGroupId();
  }

  @override
  void didUpdateWidget(covariant CurveImportExportCard oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (_selectedGroup == null) {
      _selectedGroupId = _defaultGroupId();
    }
  }

  String? _defaultGroupId() {
    final groups = _library.groups;
    if (groups.isEmpty) {
      return null;
    }
    for (final group in groups) {
      if (group.id == widget.draft.defaultCrossfaderCurve) {
        return group.id;
      }
    }
    return groups.first.id;
  }

  Future<void> _import() async {
    final group = await showImportVirtualDjCurveDialog(
      context,
      importer: widget.importer,
    );
    if (!mounted || group == null) {
      return;
    }
    final id = _uniqueId(slugifyCurveId(group.displayName), _library);
    final imported = copyCurveGroup(group, id: id);
    widget.onChanged(
      copyAppSettings(
        widget.draft,
        curveLibrary: copyCurveLibrary(
          _library,
          groups: [..._library.groups, imported],
        ),
        defaultCrossfaderCurve: imported.id,
      ),
    );
    setState(() => _selectedGroupId = imported.id);
    final l10n = AppLocalizations.of(context)!;
    showMixarToast(
      context: context,
      title: Text(l10n.curvesImported(_groupName(imported.id))),
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
    final groups = _library.groups;
    final selected = _selectedGroup;
    return SettingsSectionCard(
      title: l10n.curvesImportExportTitle,
      subtitle: l10n.curvesImportExportDescription,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        spacing: 12,
        children: [
          if (groups.isEmpty)
            Text(
              l10n.curvesEmpty,
              style: theme.typography.body.sm.copyWith(
                color: theme.colors.mutedForeground,
              ),
            )
          else
            SettingsField(
              label: l10n.curvesExportCurveLabel,
              child: SettingsSelect<String>(
                dialogTitle: l10n.curvesExportCurveLabel,
                value: selected!.id,
                options: [for (final group in groups) group.id],
                labelBuilder: _groupName,
                onChanged: (id) => setState(() => _selectedGroupId = id),
              ),
            ),
          Wrap(
            spacing: 12,
            runSpacing: 8,
            children: [
              AppButton(
                size: .sm,
                mainAxisSize: .min,
                variant: .outline,
                onPress: _import,
                child: Text(l10n.curvesImport),
              ),
              AppButton(
                size: .sm,
                mainAxisSize: .min,
                variant: .outline,
                onPress: selected == null ? null : () => _export(selected),
                child: Text(l10n.curvesExport),
              ),
            ],
          ),
        ],
      ),
    );
  }

  String _groupName(String id) {
    for (final group in _library.groups) {
      if (group.id == id) {
        return group.displayName.isEmpty ? group.id : group.displayName;
      }
    }
    return id;
  }

  String _uniqueId(String base, CurveLibrarySpec library) {
    final reserved = <String>{
      ...kReservedCurvePresetIds,
      for (final group in library.groups) group.id,
      for (final set in library.sets) set.id,
    };
    if (!reserved.contains(base)) {
      return base;
    }
    var suffix = 2;
    while (reserved.contains('${base}_$suffix')) {
      suffix++;
    }
    return '${base}_$suffix';
  }
}
