import 'dart:async';

import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/models/curve_editor.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/src/rust/api/curve_editor.dart';
import 'package:gui_flutter/src/rust/api/event_mirrors.dart';
import 'package:gui_flutter/widgets/settings/settings_field.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/mixar_dialog.dart';
import 'package:gui_flutter/widgets/shell/mixar_input.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// Bridge importer, injectable so widget tests skip the Rust runtime.
typedef VirtualDjCurveImporter = Future<VirtualDjCurveImport> Function({
  required String crossfaderCurve,
  required String crossfaderCustom,
});

/// Bridge exporter, injectable so widget tests skip the Rust runtime.
typedef VirtualDjCurveExporter = Future<VirtualDjCurveExport> Function({
  required CurveGroupSpec group,
});

/// Import a VirtualDJ curve as a native editable group.
Future<CurveGroupSpec?> showImportVirtualDjCurveDialog(
  BuildContext context, {
  VirtualDjCurveImporter? importer,
}) {
  return showMixarDialog<CurveGroupSpec>(
    context: context,
    builder: (_) =>
        _ImportVirtualDjDialogBody(importer: importer ?? _defaultImporter),
  );
}

/// Show the `crossfaderCustom` script for one native group.
Future<void> showExportVirtualDjCurveDialog(
  BuildContext context, {
  required CurveGroupSpec group,
  VirtualDjCurveExporter? exporter,
}) {
  return showMixarDialog<void>(
    context: context,
    builder: (_) => _ExportVirtualDjDialogBody(
      group: group,
      exporter: exporter ?? _defaultExporter,
    ),
  );
}

Future<VirtualDjCurveImport> _defaultImporter({
  required String crossfaderCurve,
  required String crossfaderCustom,
}) {
  return importVirtualdjCurve(
    crossfaderCurve: crossfaderCurve,
    crossfaderCustom: crossfaderCustom,
  );
}

Future<VirtualDjCurveExport> _defaultExporter({required CurveGroupSpec group}) {
  return exportVirtualdjCurve(group: group);
}

class _ImportVirtualDjDialogBody extends StatefulWidget {
  const new({required this.importer});

  final VirtualDjCurveImporter importer;

  @override
  State<_ImportVirtualDjDialogBody> createState() =>
      _ImportVirtualDjDialogBodyState();
}

class _ImportVirtualDjDialogBodyState
    extends State<_ImportVirtualDjDialogBody> {
  final _curve = TextEditingController();
  final _custom = TextEditingController();
  var _busy = false;
  String? _error;

  @override
  void dispose() {
    _curve.dispose();
    _custom.dispose();
    super.dispose();
  }

  Future<void> _import() async {
    if (_busy) {
      return;
    }
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final imported = await widget.importer(
        crossfaderCurve: _curve.text.trim(),
        crossfaderCustom: _custom.text.trim(),
      );
      if (!mounted) {
        return;
      }
      Navigator.of(context).pop(
        copyCurveGroup(imported.group, displayName: imported.suggestedName),
      );
    } on Object catch (error) {
      if (!mounted) {
        return;
      }
      setState(() {
        _error = error.toString();
        _busy = false;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final l10n = AppLocalizations.of(context)!;
    return MixarDialog(
      title: l10n.curvesImportTitle,
      description: l10n.curvesImportHelp,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        spacing: 12,
        children: [
          SettingsField(
            label: l10n.curvesImportCurveLabel,
            hint: l10n.curvesImportCurveHint,
            child: MixarInput(controller: _curve),
          ),
          SettingsField(
            label: l10n.curvesImportCustomLabel,
            hint: l10n.curvesImportCustomHint,
            child: MixarInput(controller: _custom, maxLines: 4),
          ),
          if (_error != null)
            Text(
              _error!,
              style: theme.typography.body.xs.copyWith(
                color: theme.colors.destructive,
              ),
            ),
        ],
      ),
      actions: [
        AppButton(
          variant: .outline,
          onPress: _busy ? null : () => Navigator.of(context).pop(),
          child: Text(l10n.curvesCancel),
        ),
        AppButton(
          onPress: _busy ? null : _import,
          child: Text(_busy ? l10n.curvesImporting : l10n.curvesImport),
        ),
      ],
    );
  }
}

class _ExportVirtualDjDialogBody extends StatefulWidget {
  const new({required this.group, required this.exporter});

  final CurveGroupSpec group;
  final VirtualDjCurveExporter exporter;

  @override
  State<_ExportVirtualDjDialogBody> createState() =>
      _ExportVirtualDjDialogBodyState();
}

class _ExportVirtualDjDialogBodyState
    extends State<_ExportVirtualDjDialogBody> {
  String? _script;
  String? _error;
  var _busy = true;
  var _copied = false;

  @override
  void initState() {
    super.initState();
    unawaited(_export());
  }

  Future<void> _export() async {
    try {
      final exported = await widget.exporter(group: widget.group);
      if (!mounted) {
        return;
      }
      setState(() {
        _script = exported.crossfaderCustom;
        _busy = false;
      });
    } on Object catch (error) {
      if (!mounted) {
        return;
      }
      setState(() {
        _error = error.toString();
        _busy = false;
      });
    }
  }

  Future<void> _copy() async {
    final script = _script;
    if (script == null) {
      return;
    }
    await Clipboard.setData(ClipboardData(text: script));
    if (mounted) {
      setState(() => _copied = true);
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final l10n = AppLocalizations.of(context)!;
    return MixarDialog(
      title: l10n.curvesExportTitle,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        spacing: 12,
        children: [
          if (_busy)
            Text(
              l10n.curvesExporting,
              style: theme.typography.body.sm.copyWith(
                color: theme.colors.mutedForeground,
              ),
            )
          else if (_error != null)
            Text(
              _error!,
              style: theme.typography.body.xs.copyWith(
                color: theme.colors.destructive,
              ),
            )
          else if (_script != null)
            SettingsField(
              label: l10n.curvesExportCustomLabel,
              child: DecoratedBox(
                decoration: BoxDecoration(
                  color: theme.colors.secondary,
                  borderRadius: theme.style.borderRadius.md,
                  border: Border.all(color: theme.colors.border),
                ),
                child: Padding(
                  padding: const EdgeInsets.all(10),
                  child: Text(_script!, style: theme.typography.body.sm),
                ),
              ),
            ),
        ],
      ),
      actions: _script == null
          ? const []
          : [
              AppButton(
                size: .sm,
                mainAxisSize: .min,
                variant: .outline,
                onPress: _copy,
                child: Text(_copied ? l10n.curvesCopied : l10n.curvesCopy),
              ),
              AppButton(
                size: .sm,
                mainAxisSize: .min,
                onPress: () => Navigator.of(context).pop(),
                child: Text(l10n.curvesClose),
              ),
            ],
    );
  }
}
