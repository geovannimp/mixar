import 'dart:ui' show Tristate;

import 'package:flutter/semantics.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/theme/material_theme.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/widgets/mixer/pads/hold_pad_button.dart';
import 'package:gui_flutter/widgets/mixer/pads/pad_button.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';
import 'support/pad_finders.dart';

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

  testWidgets('hover darkens the plain card face', (tester) async {
    final theme = MixarThemeData.dark();
    await pumpPad(
      tester,
      pad: PadButton(onPress: () {}, child: const Text('X')),
    );
    final pad = find.byType(PadButton);
    expect(padDecorationOf(tester, pad).color, theme.colors.card);

    final pointer = await attachMousePointer(tester);
    await pointer.moveTo(tester.getCenter(pad));
    await tester.pumpAndSettle();
    expect(padDecorationOf(tester, pad).color, theme.colors.secondary);

    await pointer.moveTo(const Offset(200, 200));
    await tester.pumpAndSettle();
    expect(padDecorationOf(tester, pad).color, theme.colors.card);
  });

  testWidgets('a latched pad fills with primary and on-primary ink', (
    tester,
  ) async {
    final theme = MixarThemeData.dark();
    await pumpPad(
      tester,
      pad: PadButton(active: true, onPress: () {}, child: const Text('X')),
    );
    final pad = find.byType(PadButton);
    expect(padDecorationOf(tester, pad).color, theme.colors.primary);
    expect(
      tester
          .widget<RichText>(
            find.descendant(of: pad, matching: find.byType(RichText)),
          )
          .text
          .style
          ?.color,
      theme.colors.primaryForeground,
    );
  });

  testWidgets('a disabled pad paints the muted face', (tester) async {
    final theme = MixarThemeData.dark();
    await pumpPad(
      tester,
      pad: const PadButton(disabled: true, child: Text('X')),
    );
    final pad = find.byType(PadButton);
    expect(
      padDecorationOf(tester, pad).color,
      theme.colors.secondary.withValues(alpha: 0.35),
    );
    expect(
      tester
          .widget<RichText>(
            find.descendant(of: pad, matching: find.byType(RichText)),
          )
          .text
          .style
          ?.color,
      theme.colors.mutedForeground,
    );
  });

  testWidgets('leaving the pad reports pointer exit', (tester) async {
    var exits = 0;
    await pumpPad(
      tester,
      pad: PadButton(
        onPress: () {},
        onPointerExit: () => exits++,
        child: const SizedBox.shrink(),
      ),
    );
    final pad = find.byType(PadButton);
    final pointer = await attachMousePointer(tester);
    await pointer.moveTo(tester.getCenter(pad));
    await tester.pumpAndSettle();
    await pointer.moveTo(const Offset(200, 200));
    await tester.pumpAndSettle();
    expect(exits, greaterThanOrEqualTo(1));
  });

  testWidgets('pressed state clears when the pad becomes disabled', (
    tester,
  ) async {
    final theme = MixarThemeData.dark();
    var disabled = false;
    late StateSetter setPad;
    await pumpPad(
      tester,
      pad: StatefulBuilder(
        builder: (context, setState) {
          setPad = setState;
          return PadButton(
            disabled: disabled,
            onPress: () {},
            child: const SizedBox.shrink(),
          );
        },
      ),
    );

    final pad = find.byType(PadButton);
    final gesture = await tester.startGesture(tester.getCenter(pad));
    await tester.pump();
    expect(padDecorationOf(tester, pad).color, theme.colors.primary);

    setPad(() => disabled = true);
    await tester.pump();
    expect(
      padDecorationOf(tester, pad).color,
      theme.colors.secondary.withValues(alpha: 0.35),
    );

    setPad(() => disabled = false);
    await tester.pump();
    expect(padDecorationOf(tester, pad).color, theme.colors.card);

    await gesture.up();
  });

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
