import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/models/smart_playlist_presets.dart';
import 'package:gui_flutter/core/models/smart_playlist_rules.dart';
import 'package:gui_flutter/core/theme/mixar_button_style.dart';
import 'package:gui_flutter/core/utils/color_labels.dart';
import 'package:gui_flutter/core/utils/key_format.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/providers/shell/app_notice.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/widgets/library/smart_preset_drawer.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/m_tappable.dart';
import 'package:gui_flutter/widgets/shell/mixar_input.dart';
import 'package:gui_flutter/widgets/shell/mixar_select.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Rule-tree editor for a smart playlist.
///
/// Every edit rebuilds an immutable [SmartPlaylistRules] and reports it through
/// [onChanged]; the widget never mutates its `initial` document.
class SmartRulesEditor extends ConsumerStatefulWidget {
  const new({
    required this.initial,
    required this.onChanged,
    this.onPresetApplied,
    this.showPresets = true,
    super.key,
  });

  final SmartPlaylistRules initial;
  final ValueChanged<SmartPlaylistRules> onChanged;

  /// Reports a preset press before the rules reach [onChanged], so a host can
  /// name a not-yet-named collection after the preset it starts from.
  final ValueChanged<SmartPlaylistRulesPreset>? onPresetApplied;

  /// Preset chips that replace the whole tree in one press.
  final bool showPresets;

  @override
  ConsumerState<SmartRulesEditor> createState() => _SmartRulesEditorState();
}

class _SmartRulesEditorState extends ConsumerState<SmartRulesEditor> {
  late SmartPlaylistRules _rules = widget.initial;

  /// Bumped when a preset replaces the tree, so every condition row is rebuilt
  /// with fresh text fields instead of keeping the previous rule's text.
  var _epoch = 0;

  /// Shown by the preset picker until a preset is applied, and again as soon as
  /// the tree is edited by hand.
  static const _presetPrompt = 'Choose a preset';

  /// Preset the picker is showing, or `null` for the prompt.
  String? _appliedPresetName;

  /// Extra option at the end of the preset list that opens the creator
  /// instead of picking a value.
  static const _addPresetOption = 'Add preset…';

  /// Bumped when the creator is opened from the picker, so the control is
  /// rebuilt from [_appliedPresetName] instead of keeping the action label.
  var _presetPickerEpoch = 0;

  /// A preset is a fixed configuration: its rules are read-only until the user
  /// forks it, so a playlist cannot drift away from the preset it names.
  bool get _locked => _appliedPresetName != null;

  /// True when the picker is shown for a document with no rules.
  bool get _treeHidden => widget.showPresets && _rules.root.rules.isEmpty;

  void _update(SmartGroupRule root) {
    final next = SmartPlaylistRules(
      root: root,
      sort: _rules.sort,
      version: _rules.version,
    );
    setState(() => _rules = next);
    widget.onChanged(next);
  }

  void _applyPreset(SmartPlaylistRulesPreset preset) {
    final next = preset.build();
    setState(() {
      _rules = next;
      _appliedPresetName = preset.name;
      _epoch += 1;
    });
    widget.onPresetApplied?.call(preset);
    widget.onChanged(next);
  }

  /// Built-ins first, then the presets this user saved.
  List<SmartPlaylistRulesPreset> _presetsWith(List<SmartPreset> custom) => [
    ...smartRulePresets,
    for (final preset in custom)
      SmartPlaylistRulesPreset(name: preset.name, build: () => preset.rules),
  ];

  void _applyPresetNamed(String name, List<SmartPlaylistRulesPreset> presets) {
    if (name == _addPresetOption) {
      // An action, not a selection: the picker goes back to whatever it was
      // showing by rebuilding its state from the model.
      setState(() => _presetPickerEpoch += 1);
      unawaited(_createPreset());
      return;
    }
    if (name == _presetPrompt) {
      // The prompt is the empty document: choose it to throw the preset's
      // configuration away and start over.
      _clearRules();
      return;
    }
    for (final preset in presets) {
      if (preset.name == name) {
        _applyPreset(preset);
        return;
      }
    }
  }

  void _clearRules() {
    const next = SmartPlaylistRules.empty();
    setState(() {
      _rules = next;
      _appliedPresetName = null;
      _epoch += 1;
    });
    widget.onChanged(next);
  }

  Future<void> _createPreset() async {
    final preset = await showSmartPresetCreatorDrawer(
      context,
      // Forking starts the draft from one of these; the tree here is read-only
      // while a preset is applied, so this is how a preset's rules change.
      presets: _presetsWith(ref.read(smartPlaylistPresetsProvider)),
    );
    if (preset == null || !mounted) {
      return;
    }
    try {
      await ref.read(smartPresetSaverProvider)(preset);
    } on Object catch (error) {
      if (mounted) {
        ref.read(appNoticeProvider.notifier).setError('$error');
      }
      return;
    }
    if (!mounted) {
      return;
    }
    _applyPreset(
      SmartPlaylistRulesPreset(name: preset.name, build: () => preset.rules),
    );
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final settings = ref
        .watch(appSettingsProvider)
        .maybeWhen(data: (value) => value, orElse: defaultAppSettings);
    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      spacing: 12,
      children: [
        if (widget.showPresets)
          Builder(
            builder: (context) {
              final presets = _presetsWith(
                ref.watch(smartPlaylistPresetsProvider),
              );
              return Row(
                spacing: 8,
                children: [
                  Text(
                    'Preset',
                    style: theme.typography.body.sm.copyWith(
                      color: theme.colors.mutedForeground,
                    ),
                  ),
                  Expanded(
                    child: MixarSelect<String>(
                      key: ValueKey('preset-$_presetPickerEpoch'),
                      value: _appliedPresetName ?? _presetPrompt,
                      options: [
                        _presetPrompt,
                        for (final preset in presets) preset.name,
                        _addPresetOption,
                      ],
                      labelBuilder: (value) => value,
                      dialogTitle: 'Preset',
                      onChanged: (value) => _applyPresetNamed(value, presets),
                    ),
                  ),
                ],
              );
            },
          ),
        // No document, no tree: with the picker in view, rules arrive from a
        // preset, so an empty document draws nothing else. The preset drawer's
        // editor (`showPresets: false`) always draws one, because that is where
        // a document is written.
        if (!_treeHidden) ...[
          _MatchRow(
            enabled: !_locked,
            match: _rules.root.match,
            onChanged: (match) =>
                _update(SmartGroupRule(match: match, rules: _rules.root.rules)),
          ),
          KeyedSubtree(
            key: ValueKey(_epoch),
            child: _GroupRules(
              group: _rules.root,
              depth: 0,
              locked: _locked,
              numDecks: settings.numDecks,
              keyDisplayMode: keyModeFromSettings(settings.keyDisplayMode),
              onChanged: _update,
            ),
          ),
        ],
      ],
    );
  }
}

/// `Match <all|any> of the following rules`.
class _MatchRow extends StatelessWidget {
  const new({
    required this.match,
    required this.onChanged,
    this.enabled = true,
  });

  final SmartMatch match;
  final ValueChanged<SmartMatch> onChanged;
  final bool enabled;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final style = theme.typography.body.sm.copyWith(
      color: theme.colors.mutedForeground,
    );
    return Row(
      spacing: 8,
      children: [
        Text('Match', style: style),
        SizedBox(
          width: 110,
          child: MixarSelect<SmartMatch>(
            value: match,
            options: SmartMatch.values,
            labelBuilder: (value) => value.label,
            dialogTitle: 'Match',
            enabled: enabled,
            onChanged: onChanged,
          ),
        ),
        Expanded(child: Text('of the following rules', style: style)),
      ],
    );
  }
}

/// One group's rules plus its add buttons; nesting recurses through
/// [_NestedGroup].
class _GroupRules extends StatelessWidget {
  const new({
    required this.group,
    required this.depth,
    required this.locked,
    required this.numDecks,
    required this.keyDisplayMode,
    required this.onChanged,
  });

  final SmartGroupRule group;

  /// Root group is depth 0; `Add group` stops before the document limit.
  final int depth;

  /// A preset's document is fixed: no rows added, none removed.
  final bool locked;
  final int numDecks;
  final KeyDisplayMode keyDisplayMode;
  final ValueChanged<SmartGroupRule> onChanged;

  static const _maxEditableDepth = 3;

  static const _newCondition = SmartConditionRule(
    field: SmartField.bpm,
    op: SmartOp.between,
    min: 120,
    max: 130,
  );

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      spacing: 8,
      children: [
        for (var index = 0; index < group.rules.length; index++)
          _rule(context, index),
        if (!locked)
          Row(
            spacing: 8,
            children: [
              AppButton(
                variant: MixarButtonVariant.outline,
                size: MixarButtonSize.xs,
                mainAxisSize: MainAxisSize.min,
                onPress: () =>
                    onChanged(_replace(group.rules.length, _newCondition)),
                child: const Text('Add rule'),
              ),
              if (depth < _maxEditableDepth)
                AppButton(
                  variant: MixarButtonVariant.outline,
                  size: MixarButtonSize.xs,
                  mainAxisSize: MainAxisSize.min,
                  onPress: () => onChanged(
                    _replace(
                      group.rules.length,
                      // Validation requires a nested group to hold a rule, so a
                      // new group arrives with one already inside.
                      const SmartGroupRule(rules: [_newCondition]),
                    ),
                  ),
                  child: const Text('Add group'),
                ),
            ],
          ),
      ],
    );
  }

  Widget _rule(BuildContext context, int index) {
    final rule = group.rules[index];
    switch (rule) {
      case final SmartConditionRule condition:
        return _ConditionRow(
          condition: condition,
          numDecks: numDecks,
          keyDisplayMode: keyDisplayMode,
          enabled: !locked,
          onChanged: (next) => onChanged(_replace(index, next)),
          onRemove: () => onChanged(_remove(index)),
        );
      case final SmartGroupRule nested:
        return _NestedGroup(
          group: nested,
          depth: depth,
          locked: locked,
          numDecks: numDecks,
          keyDisplayMode: keyDisplayMode,
          onChanged: (next) => onChanged(_replace(index, next)),
          onRemove: () => onChanged(_remove(index)),
        );
    }
  }

  SmartGroupRule _replace(int index, SmartRule rule) => SmartGroupRule(
    match: group.match,
    rules: [
      for (var i = 0; i < group.rules.length; i++)
        if (i == index) rule else group.rules[i],
      if (index >= group.rules.length) rule,
    ],
  );

  SmartGroupRule _remove(int index) => SmartGroupRule(
    match: group.match,
    rules: [
      for (var i = 0; i < group.rules.length; i++)
        if (i != index) group.rules[i],
    ],
  );
}

/// Nested group: indented, its own match row, removable.
class _NestedGroup extends StatelessWidget {
  const new({
    required this.group,
    required this.depth,
    required this.locked,
    required this.numDecks,
    required this.keyDisplayMode,
    required this.onChanged,
    required this.onRemove,
  });

  final SmartGroupRule group;
  final int depth;
  final bool locked;
  final int numDecks;
  final KeyDisplayMode keyDisplayMode;
  final ValueChanged<SmartGroupRule> onChanged;
  final VoidCallback onRemove;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return DecoratedBox(
      decoration: BoxDecoration(
        border: Border(left: BorderSide(color: theme.colors.border)),
      ),
      child: Padding(
        padding: const EdgeInsets.only(left: 12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          spacing: 8,
          children: [
            Row(
              spacing: 8,
              children: [
                Expanded(
                  child: _MatchRow(
                    enabled: !locked,
                    match: group.match,
                    onChanged: (match) => onChanged(
                      SmartGroupRule(match: match, rules: group.rules),
                    ),
                  ),
                ),
                if (!locked)
                  AppButton(
                    variant: MixarButtonVariant.ghost,
                    size: MixarButtonSize.xs,
                    onPress: onRemove,
                    child: const Text('Remove group'),
                  ),
              ],
            ),
            _GroupRules(
              group: group,
              depth: depth + 1,
              locked: locked,
              numDecks: numDecks,
              keyDisplayMode: keyDisplayMode,
              // Removing the group's last rule removes the group itself.
              onChanged: (next) =>
                  next.rules.isEmpty ? onRemove() : onChanged(next),
            ),
          ],
        ),
      ),
    );
  }
}

/// One condition: field, operator, optional deck, and the operator's values.
class _ConditionRow extends StatefulWidget {
  const new({
    required this.condition,
    required this.numDecks,
    required this.keyDisplayMode,
    required this.onChanged,
    required this.onRemove,
    this.enabled = true,
  });

  final SmartConditionRule condition;
  final int numDecks;
  final KeyDisplayMode keyDisplayMode;
  final ValueChanged<SmartConditionRule> onChanged;
  final VoidCallback onRemove;

  /// False while the row belongs to a preset: the values become read-only.
  final bool enabled;

  @override
  State<_ConditionRow> createState() => _ConditionRowState();
}

class _ConditionRowState extends State<_ConditionRow> {
  final _primary = TextEditingController();
  final _secondary = TextEditingController();

  /// Key currently chosen in the "add one key" picker (not the rule's values).
  String? _selectedKey;

  /// Text last written into a controller, so a value the model replaced can be
  /// told apart from one the user is typing.
  final Map<TextEditingController, String> _synced = {};

  @override
  void initState() {
    super.initState();
    _syncControllers(widget.condition);
  }

  @override
  void didUpdateWidget(covariant _ConditionRow oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.condition.field != widget.condition.field) {
      _selectedKey = null;
    }
    _syncControllers(widget.condition);
  }

  @override
  void dispose() {
    _primary.dispose();
    _secondary.dispose();
    super.dispose();
  }

  /// Mirror the model into the text fields.
  ///
  /// Numeric slots compare by parsed value, so typing `12.` does not rewrite
  /// the field; text slots only fix a slot the model cleared or filled
  /// elsewhere.
  void _syncControllers(SmartConditionRule condition) {
    switch (condition.op) {
      case SmartOp.between:
        if (double.tryParse(_primary.text.trim()) != condition.min) {
          _primary.text = _numberText(condition.min);
        }
        if (double.tryParse(_secondary.text.trim()) != condition.max) {
          _secondary.text = _numberText(condition.max);
        }
      case SmartOp.contains || SmartOp.equals:
        _syncText(
          _primary,
          condition.values.isEmpty ? '' : condition.values.single,
        );
      case SmartOp.inList:
        _syncText(_primary, condition.values.join(', '));
      case SmartOp.isEmpty ||
          SmartOp.isNotEmpty ||
          SmartOp.harmonicallyCompatible:
        _syncText(_primary, '');
        _syncText(_secondary, '');
    }
  }

  void _syncText(TextEditingController controller, String value) {
    final current = controller.text;
    if (current == value) {
      _synced[controller] = value;
      return;
    }
    if (current.isEmpty) {
      controller.text = value;
    } else if (value.isEmpty) {
      controller.text = '';
    } else if (current == _synced[controller]) {
      // The controller still holds what we last wrote, so the model changed
      // underneath it (a deleted sibling row reusing this slot, a preset):
      // adopt the new value. Text the user is mid-editing keeps its caret.
      controller.text = value;
    } else {
      return;
    }
    _synced[controller] = controller.text;
  }

  static String _numberText(double? value) {
    if (value == null) {
      return '';
    }
    return value == value.roundToDouble()
        ? value.round().toString()
        : value.toString();
  }

  void _onFieldChanged(SmartField field) {
    final current = widget.condition;
    if (field == current.field) {
      return;
    }
    // Operands belong to the old field: a BPM window is out of range for Rating
    // and genre text means nothing to Key, so only the operator carries over,
    // and only when the new field also accepts it.
    widget.onChanged(
      SmartConditionRule(
        field: field,
        op: field.ops.contains(current.op) ? current.op : field.ops.first,
        deckId: field.isDeck ? current.deckId : null,
      ),
    );
  }

  void _onOpChanged(SmartOp op) {
    final current = widget.condition;
    if (op == current.op) {
      return;
    }
    widget.onChanged(
      SmartConditionRule(
        field: current.field,
        op: op,
        // Bounds and values belong to the previous operator.
        deckId: current.field.isDeck ? current.deckId : null,
      ),
    );
  }

  void _onDeckChanged(int value) {
    final current = widget.condition;
    widget.onChanged(
      SmartConditionRule(
        field: current.field,
        op: current.op,
        min: current.min,
        max: current.max,
        values: current.values,
        deckId: value < 0 ? null : value,
      ),
    );
  }

  void _onPrimaryChanged(String text) {
    final current = widget.condition;
    switch (current.op) {
      case SmartOp.between:
        widget.onChanged(_withMin(current, double.tryParse(text.trim())));
      case SmartOp.contains || SmartOp.equals:
        final value = text.trim();
        widget.onChanged(
          SmartConditionRule(
            field: current.field,
            op: current.op,
            values: value.isEmpty ? const [] : [value],
          ),
        );
      case SmartOp.inList:
        widget.onChanged(
          SmartConditionRule(
            field: current.field,
            op: current.op,
            values: [
              for (final part in text.split(','))
                if (part.trim().isNotEmpty) part.trim(),
            ],
          ),
        );
      case SmartOp.isEmpty ||
          SmartOp.isNotEmpty ||
          SmartOp.harmonicallyCompatible:
        break;
    }
  }

  void _onSecondaryChanged(String text) {
    final current = widget.condition;
    widget.onChanged(_withMax(current, double.tryParse(text.trim())));
  }

  /// Each bound edits on its own: rewriting one must keep the other, because
  /// the two fields share a condition and an empty slot means "no bound".
  static SmartConditionRule _withMin(
    SmartConditionRule condition,
    double? min,
  ) => SmartConditionRule(
    field: condition.field,
    op: condition.op,
    min: min,
    max: condition.max,
    values: condition.values,
    deckId: condition.deckId,
  );

  static SmartConditionRule _withMax(
    SmartConditionRule condition,
    double? max,
  ) => SmartConditionRule(
    field: condition.field,
    op: condition.op,
    min: condition.min,
    max: max,
    values: condition.values,
    deckId: condition.deckId,
  );

  void _toggleKey(SmartConditionRule condition, String camelotCode) {
    final musical = camelotToMusical(camelotCode);
    if (musical == null) {
      return;
    }
    final values = [...condition.values];
    final slot = musicalToCamelot(musical);
    final existing = values.indexWhere(
      (value) => musicalToCamelot(value) == slot,
    );
    if (existing >= 0) {
      values.removeAt(existing);
    } else {
      values.add(musical);
    }
    widget.onChanged(
      SmartConditionRule(
        field: condition.field,
        op: condition.op,
        values: values,
      ),
    );
  }

  void _addCompatibleKeys(SmartConditionRule condition) {
    final source = _selectedKey ?? smartKeys.first;
    final values = [...condition.values];
    for (final key in harmonicFamily(source)) {
      final slot = musicalToCamelot(key);
      if (!values.any((value) => musicalToCamelot(value) == slot)) {
        values.add(key);
      }
    }
    widget.onChanged(
      SmartConditionRule(
        field: condition.field,
        op: condition.op,
        values: values,
      ),
    );
  }

  void _toggleColor(SmartConditionRule condition, LibraryColorLabel label) {
    final values = [...condition.values];
    final existing = values.indexWhere(
      (value) => value.trim().toLowerCase() == label.name,
    );
    if (existing >= 0) {
      values.removeAt(existing);
    } else {
      values.add(label.name);
    }
    widget.onChanged(
      SmartConditionRule(
        field: condition.field,
        op: condition.op,
        values: values,
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final condition = widget.condition;
    final values = _valueEditors(condition);
    final enabled = widget.enabled;
    return DecoratedBox(
      decoration: BoxDecoration(
        border: Border.all(color: theme.colors.border),
        borderRadius: theme.style.borderRadius.md,
      ),
      child: Opacity(
        opacity: enabled ? 1 : 0.6,
        child: IgnorePointer(
          ignoring: !enabled,
          child: Padding(
            padding: const EdgeInsets.all(8),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              spacing: 8,
              children: [
                Row(
                  spacing: 8,
                  children: [
                    Expanded(
                      child: MixarSelect<SmartField>(
                        value: condition.field,
                        options: SmartField.values,
                        labelBuilder: (field) => field.label,
                        dialogTitle: 'Field',
                        enabled: enabled,
                        onChanged: _onFieldChanged,
                      ),
                    ),
                    Expanded(
                      child: MixarSelect<SmartOp>(
                        value: condition.op,
                        options: condition.field.ops,
                        labelBuilder: (op) => op.label,
                        dialogTitle: 'Operator',
                        enabled: enabled,
                        onChanged: _onOpChanged,
                      ),
                    ),
                    if (enabled)
                      AppButton.icon(
                        variant: MixarButtonVariant.ghost,
                        size: MixarButtonSize.xs,
                        semanticsLabel: 'Remove rule',
                        onPress: widget.onRemove,
                        child: const Icon(LucideIcons.x),
                      ),
                  ],
                ),
                if (condition.field.isDeck)
                  MixarSelect<int>(
                    value: condition.deckId ?? -1,
                    options: [
                      -1,
                      for (var deck = 0; deck < widget.numDecks; deck++) deck,
                      // A rule saved with four decks stays editable after
                      // the app drops to two: keep the pinned deck selectable.
                      if (condition.deckId != null &&
                          condition.deckId! >= widget.numDecks)
                        condition.deckId!,
                    ],
                    labelBuilder: (value) => value < 0
                        ? 'Any playing deck'
                        : 'Deck ${String.fromCharCode(65 + value)}',
                    dialogTitle: 'Deck',
                    enabled: enabled,
                    onChanged: _onDeckChanged,
                  ),
                ...values,
              ],
            ),
          ),
        ),
      ),
    );
  }

  List<Widget> _valueEditors(SmartConditionRule condition) {
    switch (condition.op) {
      case SmartOp.between:
        final percent = condition.field == SmartField.deckBpmDelta;
        return [
          Row(
            spacing: 8,
            children: [
              Expanded(
                child: MixarInput(
                  controller: _primary,
                  hint: percent ? 'Min %' : 'Min',
                  enabled: widget.enabled,
                  keyboardType: const TextInputType.numberWithOptions(
                    decimal: true,
                    signed: true,
                  ),
                  onChanged: _onPrimaryChanged,
                ),
              ),
              Expanded(
                child: MixarInput(
                  controller: _secondary,
                  hint: percent ? 'Max %' : 'Max',
                  enabled: widget.enabled,
                  keyboardType: const TextInputType.numberWithOptions(
                    decimal: true,
                    signed: true,
                  ),
                  onChanged: _onSecondaryChanged,
                ),
              ),
            ],
          ),
        ];
      case SmartOp.contains || SmartOp.equals:
        if (condition.field == SmartField.deckKey) {
          return const [];
        }
        return [
          MixarInput(
            controller: _primary,
            hint: 'Value',
            enabled: widget.enabled,
            onChanged: _onPrimaryChanged,
          ),
        ];
      case SmartOp.inList:
        return switch (condition.field) {
          SmartField.genre || SmartField.comment => [
            MixarInput(
              controller: _primary,
              hint: 'Comma separated',
              enabled: widget.enabled,
              onChanged: _onPrimaryChanged,
            ),
          ],
          SmartField.key => _keyEditor(condition),
          SmartField.colorLabel => _colorEditor(condition),
          SmartField.title ||
          SmartField.artist ||
          SmartField.album ||
          SmartField.bpm ||
          SmartField.rating ||
          SmartField.year ||
          SmartField.durationMs ||
          SmartField.loudnessLufs ||
          SmartField.addedDaysAgo ||
          SmartField.deckBpmDelta ||
          SmartField.deckKey => const [],
        };
      case SmartOp.isEmpty ||
          SmartOp.isNotEmpty ||
          SmartOp.harmonicallyCompatible:
        return const [];
    }
  }

  List<Widget> _keyEditor(SmartConditionRule condition) {
    return [
      Wrap(
        spacing: 4,
        runSpacing: 4,
        children: [
          for (final code in smartCamelotCodes)
            _RuleChip(
              label: code,
              selected: condition.values.any(
                (value) => musicalToCamelot(value) == code,
              ),
              onPress: () => _toggleKey(condition, code),
            ),
        ],
      ),
      Row(
        spacing: 8,
        children: [
          Expanded(
            child: MixarSelect<String>(
              value: _selectedKey ?? smartKeys.first,
              options: smartKeys,
              labelBuilder: (key) => formatDeckKey(key, widget.keyDisplayMode),
              dialogTitle: 'Add key',
              enabled: widget.enabled,
              onChanged: (key) => setState(() => _selectedKey = key),
            ),
          ),
          if (widget.enabled)
            AppButton(
              variant: MixarButtonVariant.outline,
              size: MixarButtonSize.xs,
              onPress: () => _addCompatibleKeys(condition),
              child: const Text('Add compatible'),
            ),
        ],
      ),
    ];
  }

  List<Widget> _colorEditor(SmartConditionRule condition) {
    return [
      Wrap(
        spacing: 4,
        runSpacing: 4,
        children: [
          for (final label in LibraryColorLabel.values)
            _ColorChip(
              label: label,
              selected: condition.values.any(
                (value) => value.trim().toLowerCase() == label.name,
              ),
              onPress: () => _toggleColor(condition, label),
            ),
        ],
      ),
    ];
  }
}

/// Toggle chip over [MTappable] (selected fill uses the shop selection wash).
class _RuleChip extends StatelessWidget {
  const new({
    required this.label,
    required this.selected,
    required this.onPress,
  });

  final String label;
  final bool selected;
  final VoidCallback onPress;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return MTappable(
      onPress: onPress,
      selected: selected,
      semanticsLabel: label,
      builder: (context, state) {
        final fill = selected
            ? theme.colors.selection
            : state.active
            ? theme.colors.secondary
            : theme.colors.card;
        final foreground = selected
            ? theme.colors.selectionForeground
            : theme.colors.foreground;
        return DecoratedBox(
          decoration: BoxDecoration(
            color: fill,
            borderRadius: theme.style.borderRadius.sm,
            border: Border.all(
              color: theme.colors.border,
              width: theme.style.borderWidth,
            ),
          ),
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 3),
            child: Text(
              label,
              style: theme.typography.body.xs.copyWith(
                color: foreground,
                fontWeight: FontWeight.w600,
              ),
            ),
          ),
        );
      },
    );
  }
}

/// Colour-label toggle chip: swatch plus the label's display name.
class _ColorChip extends StatelessWidget {
  const new({
    required this.label,
    required this.selected,
    required this.onPress,
  });

  final LibraryColorLabel label;
  final bool selected;
  final VoidCallback onPress;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final swatch = colorLabelColor(label) ?? theme.colors.mutedForeground;
    return MTappable(
      onPress: onPress,
      selected: selected,
      semanticsLabel: colorLabelDisplayName(label),
      builder: (context, state) {
        final fill = selected
            ? theme.colors.selection
            : state.active
            ? theme.colors.secondary
            : theme.colors.card;
        final foreground = selected
            ? theme.colors.selectionForeground
            : theme.colors.foreground;
        return DecoratedBox(
          decoration: BoxDecoration(
            color: fill,
            borderRadius: theme.style.borderRadius.sm,
            border: Border.all(
              color: theme.colors.border,
              width: theme.style.borderWidth,
            ),
          ),
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 3),
            child: Row(
              mainAxisSize: MainAxisSize.min,
              spacing: 4,
              children: [
                DecoratedBox(
                  decoration: BoxDecoration(
                    color: swatch,
                    shape: BoxShape.circle,
                  ),
                  child: const SizedBox.square(dimension: 8),
                ),
                Text(
                  colorLabelDisplayName(label),
                  style: theme.typography.body.xs.copyWith(
                    color: foreground,
                    fontWeight: FontWeight.w600,
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
