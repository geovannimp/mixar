import 'dart:ui' show Tristate;

import 'package:flutter/gestures.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/theme/material_theme.dart';
import 'package:gui_flutter/core/theme/mixar_button_style.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/widgets/mixer/deck_grid_panel.dart';
import 'package:gui_flutter/widgets/mixer/mixer_button.dart';
import 'package:gui_flutter/widgets/settings/settings_numeric_stepper.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/m_tappable.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  Future<void> pumpApp(WidgetTester tester, Widget child) async {
    final theme = MixarThemeData.dark();
    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromMixar(theme),
        builder: mixarMaterialAppBuilder(theme),
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

  testWidgets('MTappable.child invokes onPress', (tester) async {
    var taps = 0;
    await pumpApp(
      tester,
      MTappable.child(onPress: () => taps++, child: const Text('Child')),
    );
    await tester.tap(find.text('Child'));
    await tester.pump();
    expect(taps, 1);
  });

  testWidgets('MTappable disabled ignores presses', (tester) async {
    const taps = 0;
    await pumpApp(
      tester,
      MTappable(builder: (_, state) => Text(state.disabled ? 'Off' : 'On')),
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
        onSecondaryPress: () => secondary++,
        builder: (_, _) => const Text('Chip'),
      ),
    );
    await tester.tap(find.text('Chip'), buttons: kSecondaryMouseButton);
    await tester.pump();
    expect(secondary, 0);
  });

  test('ghost hover paints the shared accent-picker wash', () {
    final theme = MixarThemeData.dark();
    final look = resolveMixarButtonLook(
      theme: theme,
      variant: MixarButtonVariant.ghost,
      size: MixarButtonSize.sm,
      density: MixarButtonDensity.app,
      state: const MTappableState(
        hovered: true,
        pressed: false,
        focused: false,
        disabled: false,
        selected: false,
      ),
      icon: false,
    );
    expect(look.decoration.color, theme.hoverWash);
    expect(look.decoration.color, isNot(theme.colors.selection));
  });

  testWidgets('AppButton.icon paints a circle and keeps its hit box', (
    tester,
  ) async {
    final theme = MixarThemeData.dark();
    var presses = 0;
    await pumpApp(
      tester,
      AppButton.icon(
        variant: .outline,
        size: .sm,
        semanticsLabel: 'Gear',
        onPress: () => presses++,
        child: const Icon(LucideIcons.settings, size: 16),
      ),
    );
    final button = find.byType(AppButton);
    expect(tester.getSize(button), const Size(32, 32));

    expectCircularSurface(tester, button);
    var decorations = surfaceDecorations(tester, button);
    expect(decorations, hasLength(1));
    expect(decorations.single.border, isNotNull);
    expect(decorations.single.color, theme.colors.card);

    final gesture = await tester.createGesture(kind: PointerDeviceKind.mouse);
    await gesture.addPointer(location: Offset.zero);
    addTearDown(gesture.removePointer);
    await tester.pump();
    await gesture.moveTo(tester.getCenter(button));
    await tester.pumpAndSettle();

    expectCircularSurface(tester, button);
    expect(
      surfaceDecorations(tester, button).single.color,
      theme.colors.secondary,
    );

    Focus.of(tester.element(find.byIcon(LucideIcons.settings))).requestFocus();
    await tester.pumpAndSettle();

    decorations = surfaceDecorations(tester, button);
    expect(decorations, hasLength(2));
    final focus = decorations.first;
    expect(focus.shape, BoxShape.circle);
    expect(focus.borderRadius, isNull);
    expect((focus.border! as Border).top.width, 2);
    expectCircularSurface(tester, button);

    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    await tester.pump();
    expect(presses, 1);

    await tester.tapAt(tester.getTopLeft(button) + const Offset(2, 2));
    await tester.pump();
    expect(presses, 2);
  });

  testWidgets('MixerButton.icon is circular, labeled MixerButton is a pill', (
    tester,
  ) async {
    final theme = MixarThemeData.dark();
    await pumpApp(
      tester,
      Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          MixerButton.icon(
            semanticsLabel: 'Gear',
            onPress: () {},
            child: const Icon(LucideIcons.settings),
          ),
          MixerButton(onPress: () {}, child: const Text('Mix')),
        ],
      ),
    );

    final iconButton = find.ancestor(
      of: find.byIcon(LucideIcons.settings),
      matching: find.byType(MixerButton),
    );
    expect(tester.getSize(iconButton), const Size(36, 36));
    expectCircularSurface(tester, iconButton);

    final labeled = find.ancestor(
      of: find.text('Mix'),
      matching: find.byType(MixerButton),
    );
    final labeledSurface = surfaceDecorations(tester, labeled).last;
    expect(labeledSurface.shape, BoxShape.rectangle);
    expect(labeledSurface.borderRadius, theme.style.borderRadius.pill);
  });

  testWidgets('deck grid side cells are circular and still tap', (
    tester,
  ) async {
    final theme = MixarThemeData.dark();
    var bpmUps = 0;
    await pumpApp(
      tester,
      ProviderScope(
        overrides: [
          appSettingsProvider.overrideWith((ref) async => defaultAppSettings()),
        ],
        child: DeckGridPanel(
          bpm: 120,
          hasTrack: true,
          bordered: false,
          onSetDownbeat: () {},
          onNudgeBack: () {},
          onNudgeForward: () {},
          onBpmDown: () {},
          onBpmUp: () => bpmUps++,
          onBpmSubmit: (_) {},
        ),
      ),
    );

    for (final label in const [
      'Decrease BPM',
      'Increase BPM',
      'Nudge grid back',
      'Nudge grid forward',
    ]) {
      final side = mixerButton(label);
      expect(side, findsOneWidget, reason: label);
      expectCircularSurface(tester, side, reason: label);
    }

    final now = find.ancestor(
      of: find.text('Now'),
      matching: find.byType(MixerButton),
    );
    final nowSurface = surfaceDecorations(tester, now).last;
    expect(nowSurface.shape, BoxShape.rectangle);
    expect(nowSurface.borderRadius, theme.style.borderRadius.pill);

    await tester.tap(mixerButton('Increase BPM'));
    await tester.pump();
    expect(bpmUps, 1);
  });

  testWidgets('settings numeric stepper step buttons are circular', (
    tester,
  ) async {
    final emitted = <double>[];
    await pumpApp(
      tester,
      SettingsNumericStepper(
        value: 0,
        min: 0,
        max: 2,
        step: 1,
        onChanged: emitted.add,
      ),
    );

    for (final glyph in const ['−', '+']) {
      final step = find.widgetWithText(AppButton, glyph);
      expect(step, findsOneWidget, reason: glyph);
      expectCircularSurface(tester, step, reason: glyph);
    }

    await tester.tap(find.widgetWithText(AppButton, '−'));
    await tester.pump();
    expect(emitted, isEmpty);

    await tester.tap(find.widgetWithText(AppButton, '+'));
    await tester.pump();
    expect(emitted, [1.0]);
  });
}

/// Asserts [ancestor]'s painted surface is a circle with no corner radius.
void expectCircularSurface(
  WidgetTester tester,
  Finder ancestor, {
  String? reason,
}) {
  final surface = surfaceDecorations(tester, ancestor).last;
  expect(surface.shape, BoxShape.circle, reason: reason);
  expect(surface.borderRadius, isNull, reason: reason);
}

/// Box decorations painted inside [ancestor], outermost first.
List<BoxDecoration> surfaceDecorations(WidgetTester tester, Finder ancestor) =>
    [
      for (final box in tester.widgetList<DecoratedBox>(
        find.descendant(of: ancestor, matching: find.byType(DecoratedBox)),
      ))
        box.decoration as BoxDecoration,
    ];

/// The [MixerButton] carrying [label] as its semantics label.
Finder mixerButton(String label) => find.byWidgetPredicate(
  (widget) => widget is MixerButton && widget.semanticsLabel == label,
);
