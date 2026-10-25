import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/models/smart_playlist_presets.dart';
import 'package:gui_flutter/core/models/smart_playlist_rules.dart';
import 'package:gui_flutter/widgets/library/library_form_drawer.dart';
import 'package:gui_flutter/widgets/library/smart_rules_editor.dart';
import 'package:gui_flutter/widgets/settings/settings_field.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/mixar_input.dart';
import 'package:gui_flutter/widgets/shell/mixar_select.dart';

/// Drawer width; clamped to the viewport on narrow windows.
const smartPresetDrawerWidth = 480.0;

/// Name a rule document so it appears in the preset list from now on.
///
/// [initial] pre-fills the rules, which is how "save these rules as a preset"
/// reads from the editor. [presets] are offered as starting points, so an
/// existing preset can be forked into a new name. Returns `null` when the
/// drawer is dismissed.
Future<SmartPreset?> showSmartPresetCreatorDrawer(
  BuildContext context, {
  SmartPlaylistRules initial = const SmartPlaylistRules.empty(),
  List<SmartPlaylistRulesPreset> presets = const [],
}) {
  return showLibraryFormDrawer<SmartPreset>(
    context,
    barrierLabel: 'New preset',
    width: smartPresetDrawerWidth,
    builder: (context) =>
        _SmartPresetDrawerBody(initial: initial, presets: presets),
  );
}

class _SmartPresetDrawerBody extends StatefulWidget {
  const new({required this.initial, this.presets = const []});

  final SmartPlaylistRules initial;

  /// Presets the draft can be forked from.
  final List<SmartPlaylistRulesPreset> presets;

  @override
  State<_SmartPresetDrawerBody> createState() => _SmartPresetDrawerBodyState();
}

class _SmartPresetDrawerBodyState extends State<_SmartPresetDrawerBody> {
  late final TextEditingController _nameController;
  late SmartPlaylistRules _rules = widget.initial;

  /// `Start from` option showing the rules the drawer opened with.
  static const _startFromCurrent = 'Current rules';

  String _startFrom = _startFromCurrent;

  /// Bumped when a fork replaces the draft, so the editor rebuilds from the
  /// forked document instead of keeping the previous one's text fields.
  var _draftEpoch = 0;

  @override
  void initState() {
    super.initState();
    _nameController = TextEditingController();
  }

  @override
  void dispose() {
    _nameController.dispose();
    super.dispose();
  }

  String get _name => _nameController.text.trim();

  /// A preset needs a name and rules the library would accept; a half-built
  /// document would only fail later, when a playlist is created from it.
  bool get _canSave =>
      _name.isNotEmpty &&
      _rules.root.rules.isNotEmpty &&
      _rules.validate() == null;

  /// Copy a preset, or the drawer's own rules, into the draft.
  void _startFromOption(String option) {
    final rules = option == _startFromCurrent
        ? widget.initial
        : widget.presets.firstWhere((preset) => preset.name == option).build();
    setState(() {
      _startFrom = option;
      _rules = rules;
      _draftEpoch += 1;
      // A fork keeps its source: saving under that name would replace the
      // original, so an empty name becomes "<source> copy".
      if (option != _startFromCurrent && _name.isEmpty) {
        _nameController.text = '$option copy';
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    return LibraryFormDrawerShell(
      title: 'New preset',
      subtitle: 'Starts the rule editor with these rules.',
      action: AppButton(
        onPress: _canSave
            ? () =>
                  Navigator.of(context)
                      .pop(SmartPreset(name: _name, rules: _rules))
            : null,
        child: const Text('Save preset'),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        spacing: 12,
        children: [
          SettingsField(
            label: 'Name',
            child: MixarInput(
              controller: _nameController,
              onChanged: (_) => setState(() {}),
            ),
          ),
          if (widget.presets.isNotEmpty)
            SettingsField(
              label: 'Start from',
              child: MixarSelect<String>(
                value: _startFrom,
                options: [
                  _startFromCurrent,
                  for (final preset in widget.presets) preset.name,
                ],
                labelBuilder: (value) => value,
                dialogTitle: 'Start from',
                onChanged: _startFromOption,
              ),
            ),
          SettingsField(
            label: 'Rules',
            hint: _rulesHint,
            child: KeyedSubtree(
              key: ValueKey(_draftEpoch),
              child: SmartRulesEditor(
                initial: _rules,
                // Presets inside a preset would only recurse.
                showPresets: false,
                onChanged: (next) => setState(() => _rules = next),
              ),
            ),
          ),
        ],
      ),
    );
  }

  String? get _rulesHint {
    if (_rules.root.rules.isEmpty) {
      return 'Add at least one rule.';
    }
    return _rules.validate();
  }
}
