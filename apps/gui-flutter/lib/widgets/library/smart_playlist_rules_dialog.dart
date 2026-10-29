import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/models/smart_playlist_rules.dart';
import 'package:gui_flutter/core/theme/mixar_button_style.dart';
import 'package:gui_flutter/widgets/library/smart_rules_editor.dart';
import 'package:gui_flutter/widgets/settings/settings_field.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/mixar_dialog.dart';
import 'package:gui_flutter/widgets/shell/mixar_input.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// New name plus rules document returned by [showSmartPlaylistRulesDialog].
typedef SmartPlaylistRulesResult = ({String name, SmartPlaylistRules rules});

/// Edit a smart playlist's name and rules.
///
/// The caller persists the result; nothing here touches the transport.
Future<SmartPlaylistRulesResult?> showSmartPlaylistRulesDialog(
  BuildContext context, {
  required String name,
  required SmartPlaylistRules initial,
}) {
  return showMixarDialog<SmartPlaylistRulesResult?>(
    context: context,
    builder: (context) =>
        _SmartPlaylistRulesDialogBody(name: name, initial: initial),
  );
}

class _SmartPlaylistRulesDialogBody extends StatefulWidget {
  const new({required this.name, required this.initial});

  final String name;
  final SmartPlaylistRules initial;

  @override
  State<_SmartPlaylistRulesDialogBody> createState() =>
      _SmartPlaylistRulesDialogBodyState();
}

class _SmartPlaylistRulesDialogBodyState
    extends State<_SmartPlaylistRulesDialogBody> {
  late final TextEditingController _nameController;
  late SmartPlaylistRules _rules = widget.initial;

  @override
  void initState() {
    super.initState();
    _nameController = TextEditingController(text: widget.name);
  }

  @override
  void dispose() {
    _nameController.dispose();
    super.dispose();
  }

  String get _name => _nameController.text.trim();

  /// A cleared tree (the preset prompt) is not a playlist: saving it would
  /// replace the rules with an empty document.
  bool get _canSave =>
      _name.isNotEmpty &&
      _rules.root.rules.isNotEmpty &&
      _rules.validate() == null;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return Padding(
      padding: const EdgeInsets.all(16),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        spacing: 12,
        children: [
          Text(
            'Smart playlist',
            style: theme.typography.body.md.copyWith(
              fontWeight: FontWeight.w700,
            ),
          ),
          SettingsField(
            label: 'Name',
            child: MixarInput(
              controller: _nameController,
              onChanged: (_) => setState(() {}),
            ),
          ),
          SettingsField(
            label: 'Rules',
            hint: _rules.validate(),
            child: SmartRulesEditor(
              initial: _rules,
              onChanged: (next) => setState(() => _rules = next),
            ),
          ),
          Row(
            spacing: 8,
            children: [
              AppButton(
                variant: MixarButtonVariant.outline,
                onPress: () => Navigator.of(context).pop(),
                child: const Text('Cancel'),
              ),
              AppButton(
                onPress: _canSave
                    ? () =>
                          Navigator.of(context)
                              .pop((name: _name, rules: _rules))
                    : null,
                child: const Text('Save'),
              ),
            ],
          ),
        ],
      ),
    );
  }
}
