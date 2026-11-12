import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/models/curve_editor.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/src/rust/api/event_mirrors.dart';
import 'package:gui_flutter/widgets/settings/settings_field.dart';
import 'package:gui_flutter/widgets/settings/settings_select.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/mixar_dialog.dart';
import 'package:gui_flutter/widgets/shell/mixar_input.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

const _scalarControl = 'scalar';
const _abControl = 'ab';

/// Cancel + dialog-specific confirm button for the curve dialogs.
List<Widget> _curveDialogActions({
  required BuildContext context,
  required String cancelLabel,
  required Widget confirm,
}) {
  return [
    AppButton(
      variant: .outline,
      onPress: () => Navigator.of(context).pop(),
      child: Text(cancelLabel),
    ),
    confirm,
  ];
}

/// Create a custom curve group with a stable id.
Future<CurveGroupSpec?> showCreateCurveGroupDialog(
  BuildContext context, {
  required CurveLibrarySpec library,
}) {
  return showMixarDialog<CurveGroupSpec>(
    context: context,
    builder: (_) => _CurveGroupDialogBody(library: library),
  );
}

/// Rename a saved curve without changing its id.
Future<String?> showRenameCurveGroupDialog(
  BuildContext context, {
  required CurveGroupSpec group,
}) {
  return showMixarDialog<String>(
    context: context,
    builder: (_) => _RenameDialogBody(
      title: AppLocalizations.of(context)!.curvesRenameTitle,
      initialName: group.displayName.isEmpty ? group.id : group.displayName,
      id: group.id,
    ),
  );
}

/// Create a two-member curve set ready for the morph editor.
Future<CurveSetSpec?> showCreateCurveSetDialog(
  BuildContext context, {
  required CurveLibrarySpec library,
}) {
  return showMixarDialog<CurveSetSpec>(
    context: context,
    builder: (_) => _CurveSetDialogBody(library: library),
  );
}

/// Rename a saved curve set without changing its id or members.
Future<String?> showRenameCurveSetDialog(
  BuildContext context, {
  required CurveSetSpec set,
}) {
  return showMixarDialog<String>(
    context: context,
    builder: (_) => _RenameDialogBody(
      title: AppLocalizations.of(context)!.curvesRenameSetTitle,
      initialName: set.displayName.isEmpty ? set.id : set.displayName,
      id: set.id,
    ),
  );
}

class _CurveGroupDialogBody extends StatefulWidget {
  const new({required this.library});

  final CurveLibrarySpec library;

  @override
  State<_CurveGroupDialogBody> createState() => _CurveGroupDialogBodyState();
}

class _CurveGroupDialogBodyState extends State<_CurveGroupDialogBody> {
  final _name = TextEditingController();
  final _id = TextEditingController();
  var _idEdited = false;
  String _control = _abControl;

  @override
  void dispose() {
    _name.dispose();
    _id.dispose();
    super.dispose();
  }

  String? get _idError => curveIdError(
    _id.text.trim(),
    otherGroupIds: widget.library.groups.map((group) => group.id),
    otherSetIds: widget.library.sets.map((set) => set.id),
  );

  bool get _canCreate =>
      _name.text.trim().isNotEmpty &&
      _id.text.trim().isNotEmpty &&
      _idError == null;

  void _nameChanged(String value) {
    if (!_idEdited) {
      _id.text = slugifyCurveId(value);
    }
    setState(() {});
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return MixarDialog(
      title: l10n.curvesCreateTitle,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        spacing: 12,
        children: [
          SettingsField(
            label: l10n.curvesName,
            child: MixarInput(controller: _name, onChanged: _nameChanged),
          ),
          SettingsField(
            label: l10n.curvesId,
            hint: l10n.curvesIdHint,
            child: MixarInput(
              controller: _id,
              onChanged: (_) {
                _idEdited = true;
                setState(() {});
              },
              errorText: _idError,
            ),
          ),
          SettingsField(
            label: l10n.curvesControl,
            child: SettingsSelect<String>(
              dialogTitle: l10n.curvesControl,
              value: _control,
              options: const [_abControl, _scalarControl],
              labelBuilder: (value) => value == _abControl
                  ? l10n.curvesControlAb
                  : l10n.curvesControlScalar,
              onChanged: (value) => setState(() => _control = value),
            ),
          ),
        ],
      ),
      actions: _curveDialogActions(
        context: context,
        cancelLabel: l10n.curvesCancel,
        confirm: AppButton(
          onPress: _canCreate
              ? () {
                  Navigator.of(context).pop(
                    newCurveGroup(
                      id: _id.text.trim(),
                      displayName: _name.text.trim(),
                      channels: _control == _abControl ? 2 : 1,
                    ),
                  );
                }
              : null,
          child: Text(l10n.curvesCreate),
        ),
      ),
    );
  }
}

class _CurveSetDialogBody extends StatefulWidget {
  const new({required this.library});

  final CurveLibrarySpec library;

  @override
  State<_CurveSetDialogBody> createState() => _CurveSetDialogBodyState();
}

class _CurveSetDialogBodyState extends State<_CurveSetDialogBody> {
  final _name = TextEditingController();
  final _id = TextEditingController();
  var _idEdited = false;

  @override
  void dispose() {
    _name.dispose();
    _id.dispose();
    super.dispose();
  }

  String? get _idError => curveIdError(
    _id.text.trim(),
    otherGroupIds: widget.library.groups.map((group) => group.id),
    otherSetIds: widget.library.sets.map((set) => set.id),
  );

  bool get _canCreate =>
      _name.text.trim().isNotEmpty &&
      _id.text.trim().isNotEmpty &&
      _idError == null;

  void _nameChanged(String value) {
    if (!_idEdited) {
      _id.text = slugifyCurveId(value);
    }
    setState(() {});
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return MixarDialog(
      title: l10n.curvesCreateSetTitle,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        spacing: 12,
        children: [
          SettingsField(
            label: l10n.curvesName,
            child: MixarInput(controller: _name, onChanged: _nameChanged),
          ),
          SettingsField(
            label: l10n.curvesId,
            hint: l10n.curvesIdHint,
            child: MixarInput(
              controller: _id,
              onChanged: (_) {
                _idEdited = true;
                setState(() {});
              },
              errorText: _idError,
            ),
          ),
        ],
      ),
      actions: _curveDialogActions(
        context: context,
        cancelLabel: l10n.curvesCancel,
        confirm: AppButton(
          onPress: _canCreate
              ? () {
                  Navigator.of(context).pop(
                    newCurveSet(
                      id: _id.text.trim(),
                      displayName: _name.text.trim(),
                    ),
                  );
                }
              : null,
          child: Text(l10n.curvesCreate),
        ),
      ),
    );
  }
}

class _RenameDialogBody extends StatefulWidget {
  const new({required this.title, required this.initialName, required this.id});

  final String title;
  final String initialName;
  final String id;

  @override
  State<_RenameDialogBody> createState() => _RenameDialogBodyState();
}

class _RenameDialogBodyState extends State<_RenameDialogBody> {
  late final TextEditingController _name = TextEditingController(
    text: widget.initialName,
  );

  @override
  void dispose() {
    _name.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final l10n = AppLocalizations.of(context)!;
    return MixarDialog(
      title: widget.title,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        spacing: 12,
        children: [
          SettingsField(
            label: l10n.curvesName,
            child: MixarInput(
              controller: _name,
              onChanged: (_) => setState(() {}),
            ),
          ),
          Text(
            widget.id,
            style: theme.typography.body.xs.copyWith(
              color: theme.colors.mutedForeground,
            ),
          ),
        ],
      ),
      actions: _curveDialogActions(
        context: context,
        cancelLabel: l10n.curvesCancel,
        confirm: AppButton(
          onPress: _name.text.trim().isEmpty
              ? null
              : () => Navigator.of(context).pop(_name.text.trim()),
          child: Text(l10n.curvesSave),
        ),
      ),
    );
  }
}
