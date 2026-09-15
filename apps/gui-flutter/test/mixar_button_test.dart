import 'dart:ui' show Tristate;

import 'package:flutter/gestures.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/mixer_button.dart';
import 'package:gui_flutter/shell/app_button.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:gui_flutter/shell/m_tappable.dart';
import 'package:material_ui/material_ui.dart';

import 'support/forui_material_app.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

void main() {
  Future<void> pumpApp(WidgetTester tester, Widget child) async {
    final theme = FTheme.neutral.dark.desktop;
    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromForui(theme),
        builder: foruiMaterialAppBuilder(theme),
        home: Scaffold(body: Center(child: child)),
      ),
    );
  }

  testWidgets('MTappable invokes onPress', (tester) async {
    var taps = 0;
    await pumpApp(
      tester,
      MTappable(onPress: () => taps++, builder: (_, _) => const Text('Tap')),
    );
    await tester.tap(find.text('Tap'));
    await tester.pump();
    expect(taps, 1);
  });

  testWidgets('MTappable disabled ignores presses', (tester) async {
    var taps = 0;
    await pumpApp(
      tester,
      MTappable(
        onPress: null,
        builder: (_, state) => Text(state.disabled ? 'Off' : 'On'),
      ),
    );
    expect(find.text('Off'), findsOneWidget);
    await tester.tap(find.text('Off'));
    await tester.pump();
    expect(taps, 0);
  });

  testWidgets('AppButton and MixerButton fire onPress', (tester) async {
    var app = 0;
    var mix = 0;
    await pumpApp(
      tester,
      Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          AppButton(onPress: () => app++, child: const Text('App')),
          MixerButton(onPress: () => mix++, child: const Text('Mix')),
        ],
      ),
    );
    await tester.tap(find.text('App'));
    await tester.tap(find.text('Mix'));
    await tester.pump();
    expect(app, 1);
    expect(mix, 1);
  });

  testWidgets('MixerButton.icon is tappable', (tester) async {
    var taps = 0;
    await pumpApp(
      tester,
      MixerButton.icon(
        semanticsLabel: 'Gear',
        onPress: () => taps++,
        child: const Icon(LucideIcons.settings),
      ),
    );
    await tester.tap(find.byIcon(LucideIcons.settings));
    await tester.pump();
    expect(taps, 1);
  });

  testWidgets('selected latched button reports Semantics.selected', (
    tester,
  ) async {
    final handle = tester.ensureSemantics();
    try {
      await pumpApp(
        tester,
        MixerButton(
          selected: true,
          onPress: () {},
          semanticsLabel: 'Cue',
          child: const Text('Cue'),
        ),
      );
      final flags = tester
          .getSemantics(find.text('Cue'))
          .getSemanticsData()
          .flagsCollection;
      expect(flags.isSelected, Tristate.isTrue);
      expect(flags.isButton, isTrue);
    } finally {
      handle.dispose();
    }
  });

  testWidgets('disabled ignores secondary press', (tester) async {
    var secondary = 0;
    await pumpApp(
      tester,
      MTappable(
        onPress: null,
        onSecondaryPress: () => secondary++,
        builder: (_, _) => const Text('Chip'),
      ),
    );
    await tester.tap(find.text('Chip'), buttons: kSecondaryMouseButton);
    await tester.pump();
    expect(secondary, 0);
  });
}
