//! Rule editor: appends/removes rules, swaps operators with the field, pins a
//! deck, and applies presets. Assertions read the emitted documents, which are
//! the exact JSON the transport receives.

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/models/smart_playlist_presets.dart';
import 'package:gui_flutter/core/models/smart_playlist_rules.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/widgets/library/library_form_drawer.dart';
import 'package:gui_flutter/widgets/library/smart_rules_editor.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';
import 'package:shadcn_ui/shadcn_ui.dart';

import 'support/mixar_material_app.dart';

const _singleCondition = SmartPlaylistRules(
  root: SmartGroupRule(
    rules: [
      SmartConditionRule(
        field: SmartField.title,
        op: SmartOp.contains,
        values: ['house'],
      ),
    ],
  ),
);

Future<List<SmartPlaylistRules>> _pumpEditor(
  WidgetTester tester, {
  SmartPlaylistRules initial = const SmartPlaylistRules.empty(),
  List<SmartPreset> presets = const [],
  SmartPresetSaver? savePreset,
  bool showPresets = true,
}) async {
  tester.view.physicalSize = const Size(1000, 1800);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  final changes = <SmartPlaylistRules>[];
  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        appSettingsProvider.overrideWith((ref) async => defaultAppSettings()),
        smartPlaylistPresetsProvider.overrideWithValue(presets),
        if (savePreset != null)
          smartPresetSaverProvider.overrideWithValue(savePreset),
      ],
      child: mixarTestMaterialApp(
        theme: MixarThemeData.dark(),
        home: Scaffold(
          body: SingleChildScrollView(
            child: SizedBox(
              width: 720,
              child: SmartRulesEditor(
                initial: initial,
                showPresets: showPresets,
                onChanged: changes.add,
              ),
            ),
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
  return changes;
}

/// Open the select whose collapsed value reads [trigger] and pick [option].
///
/// Selects are driven by their visible text: the collapsed value is the first
/// match, the option list renders after it once the menu opens.
Future<void> _choose(
  WidgetTester tester, {
  required String trigger,
  required String option,
}) async {
  await tester.tap(find.text(trigger).first);
  await tester.pumpAndSettle();
  // Long option lists scroll inside the menu; bring the entry into view first
  // or the tap lands on the clipped menu background.
  final target = find.text(option).last;
  await tester.ensureVisible(target);
  await tester.pumpAndSettle();
  await tester.tap(target);
  await tester.pumpAndSettle();
}

void main() {
  testWidgets('appends and removes rules', (tester) async {
    // The preset drawer's editor: the one place an empty tree is editable.
    final changes = await _pumpEditor(tester, showPresets: false);
    expect(find.text('Add rule'), findsOneWidget);

    await tester.tap(find.text('Add rule'));
    await tester.pumpAndSettle();
    expect(changes, hasLength(1));
    expect(changes.last.root.rules, hasLength(1));
    final added = changes.last.root.rules.single as SmartConditionRule;
    expect(added.field, SmartField.bpm);
    expect(changes.last.validate(), isNull);

    await tester.tap(find.byIcon(LucideIcons.x));
    await tester.pumpAndSettle();
    expect(changes.last.root.rules, isEmpty);
  });

  testWidgets('a new group arrives with one rule', (tester) async {
    final changes = await _pumpEditor(tester, showPresets: false);
    await tester.tap(find.text('Add group'));
    await tester.pumpAndSettle();

    final group = changes.last.root.rules.single as SmartGroupRule;
    expect(group.rules, hasLength(1));
    expect(changes.last.validate(), isNull);

    // Removing the group's last rule removes the group itself.
    await tester.tap(find.byIcon(LucideIcons.x));
    await tester.pumpAndSettle();
    expect(changes.last.root.rules, isEmpty);
  });

  testWidgets('switching to deck_key swaps the operator and drops stale '
      'values', (tester) async {
    final changes = await _pumpEditor(tester, initial: _singleCondition);

    await _choose(tester, trigger: 'Title', option: 'Deck key');

    final condition = changes.last.root.rules.single as SmartConditionRule;
    expect(condition.field, SmartField.deckKey);
    expect(condition.op, SmartOp.equals);
    expect(condition.values, isEmpty);
    expect(condition.min, isNull);
    expect(condition.max, isNull);
    // Only the deck operators remain, and the deck picker appears.
    await tester.tap(find.text('is').first);
    await tester.pumpAndSettle();
    expect(find.text('is harmonically compatible'), findsWidgets);
    expect(find.text('is between'), findsNothing);
    await tester.tap(find.text('is harmonically compatible').last);
    await tester.pumpAndSettle();
    expect(
      (changes.last.root.rules.single as SmartConditionRule).op,
      SmartOp.harmonicallyCompatible,
    );
    expect(find.text('Any playing deck'), findsOneWidget);
  });

  testWidgets('deck picker maps "any playing deck" to an absent selector', (
    tester,
  ) async {
    final changes = await _pumpEditor(tester, initial: _singleCondition);
    await _choose(tester, trigger: 'Title', option: 'Deck key');
    final condition = changes.last.root.rules.single as SmartConditionRule;
    expect(condition.toJson().containsKey('deck'), isFalse);

    await _choose(tester, trigger: 'Any playing deck', option: 'Deck A');
    final pinned = changes.last.root.rules.single as SmartConditionRule;
    expect(pinned.deckId, 0);
    expect(pinned.toJson()['deck'], 0);

    await _choose(tester, trigger: 'Deck A', option: 'Any playing deck');
    final unpinned = changes.last.root.rules.single as SmartConditionRule;
    expect(unpinned.deckId, isNull);
  });

  testWidgets('editing one bound keeps the other', (tester) async {
    final changes = await _pumpEditor(
      tester,
      initial: const SmartPlaylistRules(
        root: SmartGroupRule(
          rules: [
            SmartConditionRule(
              field: SmartField.bpm,
              op: SmartOp.between,
              min: 120,
              max: 130,
            ),
          ],
        ),
      ),
    );
    expect(find.byType(ShadInput), findsNWidgets(2));

    await tester.enterText(find.byType(ShadInput).first, '125');
    await tester.pumpAndSettle();
    var condition = changes.last.root.rules.single as SmartConditionRule;
    expect(condition.min, 125);
    expect(condition.max, 130);

    await tester.enterText(find.byType(ShadInput).last, '135');
    await tester.pumpAndSettle();
    condition = changes.last.root.rules.single as SmartConditionRule;
    expect(condition.min, 125);
    expect(condition.max, 135);
  });

  testWidgets('a field switch with a shared operator drops the operands', (
    tester,
  ) async {
    final changes = await _pumpEditor(
      tester,
      initial: const SmartPlaylistRules(
        root: SmartGroupRule(
          rules: [
            SmartConditionRule(
              field: SmartField.bpm,
              op: SmartOp.between,
              min: 120,
              max: 130,
            ),
          ],
        ),
      ),
    );

    await _choose(tester, trigger: 'BPM', option: 'Rating');

    final condition = changes.last.root.rules.single as SmartConditionRule;
    expect(condition.field, SmartField.rating);
    expect(condition.op, SmartOp.between, reason: 'the operator is shared');
    expect(condition.min, isNull, reason: 'a BPM window is not a rating');
    expect(condition.max, isNull);
    // Mid-edit the row is incomplete; the drawer's Create gate blocks it until
    // a bound is typed.
    expect(changes.last.validate(), 'between needs min or max on rating');
  });

  testWidgets('a pinned deck beyond the configured count stays editable', (
    tester,
  ) async {
    // Default settings are two decks; this rule was saved with four.
    await _pumpEditor(
      tester,
      initial: const SmartPlaylistRules(
        root: SmartGroupRule(
          rules: [
            SmartConditionRule(
              field: SmartField.deckKey,
              op: SmartOp.equals,
              deckId: 3,
            ),
          ],
        ),
      ),
    );

    expect(find.text('Deck D'), findsOneWidget);
    await tester.tap(find.text('Deck D').first);
    await tester.pumpAndSettle();
    expect(
      find.text('Deck D'),
      findsNWidgets(2),
      reason: 'the pinned deck is offered in the list, not only as the label',
    );
  });

  testWidgets('a preset picked from the list replaces the tree', (
    tester,
  ) async {
    final changes = await _pumpEditor(tester, initial: _singleCondition);
    final preset = smartRulePresets.firstWhere(
      (candidate) => candidate.name == '±6% + harmonic key',
    );
    expect(find.text('Choose a preset'), findsOneWidget);

    await _choose(
      tester,
      trigger: 'Choose a preset',
      option: '±6% + harmonic key',
    );

    expect(changes.last.toJsonString(), preset.build().toJsonString());
    expect(changes.last.usesDeckReference, isTrue);
    expect(find.text('±6% + harmonic key'), findsOneWidget);
    expect(find.text('Deck BPM (±%)'), findsOneWidget);
    expect(find.text('Deck key'), findsOneWidget);
  });

  testWidgets('picking the prompt clears the preset configuration', (
    tester,
  ) async {
    final changes = await _pumpEditor(tester);
    await _choose(
      tester,
      trigger: 'Choose a preset',
      option: '±6% + harmonic key',
    );
    expect(changes.last.root.rules, hasLength(2));

    await _choose(
      tester,
      trigger: '±6% + harmonic key',
      option: 'Choose a preset',
    );

    expect(changes.last.root.rules, isEmpty);
    expect(changes.last.sort, isNull, reason: 'the preset ordering goes too');
    expect(find.text('Choose a preset'), findsOneWidget);
    expect(find.text('Add at least one rule.'), findsNothing);
  });

  testWidgets('a saved preset is offered and applies its rules', (
    tester,
  ) async {
    const custom = SmartPreset(
      name: 'My peak',
      rules: SmartPlaylistRules(
        root: SmartGroupRule(
          rules: [
            SmartConditionRule(
              field: SmartField.bpm,
              op: SmartOp.between,
              min: 140,
              max: 150,
            ),
          ],
        ),
      ),
    );
    final changes = await _pumpEditor(tester, presets: const [custom]);

    await _choose(tester, trigger: 'Choose a preset', option: 'My peak');

    expect(changes.last.toJsonString(), custom.rules.toJsonString());
    expect(find.text('My peak'), findsOneWidget);
  });

  testWidgets('Add preset saves the current rules and applies the preset', (
    tester,
  ) async {
    final saved = <SmartPreset>[];
    final changes = await _pumpEditor(
      tester,
      initial: _singleCondition,
      savePreset: (preset) async => saved.add(preset),
    );

    await _choose(tester, trigger: 'Choose a preset', option: 'Add preset…');
    expect(find.text('New preset'), findsOneWidget);
    final save = find.widgetWithText(AppButton, 'Save preset');
    expect(
      tester.widget<AppButton>(save).onPress,
      isNull,
      reason: 'a preset needs a name',
    );

    // The editor stays mounted behind the drawer, so scope to the drawer.
    final nameField = find
        .descendant(
          of: find.byType(LibraryFormDrawerShell),
          matching: find.byType(ShadInput),
        )
        .first;
    await tester.enterText(nameField, '  House only ');
    await tester.pumpAndSettle();
    await tester.tap(save);
    await tester.pumpAndSettle();
    expect(find.text('New preset'), findsNothing);

    expect(saved.single.name, 'House only');
    expect(saved.single.rules.toJsonString(), _singleCondition.toJsonString());
    expect(changes.last.toJsonString(), _singleCondition.toJsonString());
    expect(find.text('House only'), findsOneWidget);
  });

  testWidgets('cancelling the preset creator keeps the picker value', (
    tester,
  ) async {
    final saved = <SmartPreset>[];
    final changes = await _pumpEditor(
      tester,
      initial: _singleCondition,
      savePreset: (preset) async => saved.add(preset),
    );

    await _choose(tester, trigger: 'Choose a preset', option: 'Add preset…');
    expect(find.text('New preset'), findsOneWidget);

    await tester.tapAt(const Offset(8, 8));
    await tester.pumpAndSettle();

    expect(saved, isEmpty);
    expect(find.text('New preset'), findsNothing);
    expect(
      find.text('Add preset…'),
      findsNothing,
      reason: 'an action is not a selection: the picker shows its value',
    );
    expect(find.text('Choose a preset'), findsOneWidget);
    expect(changes, isEmpty);
  });

  testWidgets('a preset keeps its rules read-only until it is forked', (
    tester,
  ) async {
    final changes = await _pumpEditor(tester);
    await _choose(
      tester,
      trigger: 'Choose a preset',
      option: '±6% + harmonic key',
    );

    final inputs = find.byType(ShadInput);
    expect(inputs, findsNWidgets(2));
    for (final input in tester.widgetList<ShadInput>(inputs)) {
      expect(
        input.enabled,
        isFalse,
        reason: 'a preset is a fixed configuration',
      );
    }
    expect(find.text('Add rule'), findsNothing);
    expect(find.text('Add group'), findsNothing);

    final window = changes.last.root.rules.first as SmartConditionRule;
    expect(window.field, SmartField.deckBpmDelta);
    expect(window.min, -6);
    expect(window.max, 6);
  });

  testWidgets('an empty document draws only the preset picker', (tester) async {
    await _pumpEditor(tester);

    expect(find.text('Choose a preset'), findsOneWidget);
    expect(
      find.text('Match'),
      findsNothing,
      reason: 'rules arrive with a preset, not from an empty tree',
    );
    expect(find.text('Add rule'), findsNothing);
    expect(find.text('Add group'), findsNothing);
    expect(find.byType(ShadInput), findsNothing);
  });

  testWidgets('forking a preset copies its rules and names the copy', (
    tester,
  ) async {
    final saved = <SmartPreset>[];
    final changes = await _pumpEditor(
      tester,
      initial: _singleCondition,
      savePreset: (preset) async => saved.add(preset),
    );

    await _choose(tester, trigger: 'Choose a preset', option: 'Add preset…');
    await _choose(
      tester,
      trigger: 'Current rules',
      option: '±6% + harmonic key',
    );

    final nameField = find
        .descendant(
          of: find.byType(LibraryFormDrawerShell),
          matching: find.byType(ShadInput),
        )
        .first;
    expect(
      tester.widget<ShadInput>(nameField).controller?.text,
      '±6% + harmonic key copy',
      reason: 'saving over the source would rename it, not fork it',
    );

    await tester.tap(find.widgetWithText(AppButton, 'Save preset'));
    await tester.pumpAndSettle();

    final window = smartRulePresets
        .firstWhere((preset) => preset.name == '±6% + harmonic key')
        .build();
    expect(saved.single.name, '±6% + harmonic key copy');
    expect(saved.single.rules.toJsonString(), window.toJsonString());
    expect(changes.last.toJsonString(), window.toJsonString());
    expect(find.text('±6% + harmonic key copy'), findsOneWidget);
  });
}
