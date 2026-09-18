import 'dart:ui' show Tristate;

import 'package:flutter/semantics.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/mixer/pads/pad_button.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_providers.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  Future<void> pumpPad(WidgetTester tester, {required Widget pad}) async {
    final theme = MixarThemeData.dark();
    final settings = copyAppSettings(defaultAppSettings(), showTooltips: true);
    await tester.pumpWidget(
      ProviderScope(
        overrides: [appSettingsProvider.overrideWith((ref) async => settings)],
        child: MaterialApp(
          theme: materialUiThemeFromMixar(theme),
          builder: mixarMaterialAppBuilder(theme),
          home: Scaffold(body: SizedBox(width: 80, height: 80, child: pad)),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  testWidgets('tooltip PadButton exposes tap action that fires onPress', (
    tester,
  ) async {
    final semantics = tester.ensureSemantics();
    try {
      var presses = 0;
      await pumpPad(
        tester,
        pad: PadButton(
          tooltip: 'Hot cue 1',
          onPress: () => presses++,
          child: const Text('1'),
        ),
      );

      final data = find.semantics
          .byLabel('Hot cue 1')
          .evaluate()
          .single
          .getSemanticsData();
      expect(data.flagsCollection.isButton, isTrue);
      expect(data.flagsCollection.isEnabled, Tristate.isTrue);
      expect(data.hasAction(SemanticsAction.tap), isTrue);

      tester.semantics.tap(find.semantics.byLabel('Hot cue 1'));
      expect(presses, 1);
    } finally {
      semantics.dispose();
    }
  });

  testWidgets('tooltip HoldPadButton semantic tap runs begin then end', (
    tester,
  ) async {
    final semantics = tester.ensureSemantics();
    try {
      var begins = 0;
      var ends = 0;
      await pumpPad(
        tester,
        pad: HoldPadButton(
          tooltip: 'Cue hold',
          onBegin: () => begins++,
          onEnd: () => ends++,
          child: const Text('H'),
        ),
      );

      tester.semantics.tap(find.semantics.byLabel('Cue hold'));
      expect(begins, 1);
      expect(ends, 1);
    } finally {
      semantics.dispose();
    }
  });

  testWidgets('disabled tooltip pad has no tap action', (tester) async {
    final semantics = tester.ensureSemantics();
    try {
      var presses = 0;
      await pumpPad(
        tester,
        pad: PadButton(
          tooltip: 'Muted',
          disabled: true,
          onPress: () => presses++,
          child: const Text('X'),
        ),
      );

      final data = find.semantics
          .byLabel('Muted')
          .evaluate()
          .single
          .getSemanticsData();
      expect(data.flagsCollection.isEnabled, Tristate.isFalse);
      expect(data.hasAction(SemanticsAction.tap), isFalse);
      expect(presses, 0);
    } finally {
      semantics.dispose();
    }
  });
}
