import 'dart:typed_data';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/theme/material_theme.dart';
import 'package:gui_flutter/core/utils/fx_params.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/src/rust/api/engine.dart';
import 'package:gui_flutter/widgets/mixer/fx_bar.dart';
import 'package:gui_flutter/widgets/mixer/mixer_button.dart';
import 'package:gui_flutter/widgets/shell/mixar_slider.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:material_ui/material_ui.dart';

import 'support/fake_engine_transport.dart';
import 'support/mixar_material_app.dart';

/// Echo as the engine publishes it; `divisionNorm` is param 0 (0 = 1/16 … 1 = 2/1).
FxSlotChrome echoSlot({bool enabled = false, double divisionNorm = 0.5}) =>
    FxSlotChrome(
      kind: FxKind.echo,
      enabled: enabled,
      wet: 0.5,
      params: Float32List.fromList([divisionNorm, 0.5, 0.0]),
    );

FxSlotChrome reverbSlot() => FxSlotChrome(
  kind: FxKind.reverb,
  enabled: false,
  wet: 0.5,
  params: Float32List.fromList([0.5, 0.5, 0.0]),
);

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  Future<({ProviderContainer container, FakeEngineTransport engine})> pumpBar(
    WidgetTester tester, {
    bool running = true,
    List<FxSlotChrome>? fxSlots,
    double width = 640,
  }) async {
    final engine = FakeEngineTransport();
    final container = ProviderContainer(
      overrides: [
        engineTransportProvider.overrideWith((ref) async => engine),
        appSettingsProvider.overrideWith((ref) async => defaultAppSettings()),
      ],
    );
    addTearDown(container.dispose);
    // A null fx list leaves the shared unit untouched, like a real evt would.
    container.read(engineUiProvider.notifier)
      ..apply(EngineEvt(kind: EngineEvtKind.status, running: running))
      ..apply(
        EngineEvt(
          kind: EngineEvtKind.updated,
          deckId: 0,
          fx: fxSlots,
          fxRouted: true,
        ),
      );

    final theme = MixarThemeData.dark();
    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: MaterialApp(
          theme: materialUiThemeFromMixar(theme),
          builder: mixarMaterialAppBuilder(theme),
          home: Scaffold(
            body: SizedBox(
              width: width,
              height: FxBar.height,
              child: const FxBar(),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    return (container: container, engine: engine);
  }

  Future<void> openEffectMenu(WidgetTester tester) async {
    await tester.tap(find.byIcon(LucideIcons.chevronDown));
    await tester.pumpAndSettle();
  }

  testWidgets('is one slim strip with the Beat FX controls', (tester) async {
    await pumpBar(tester);

    expect(find.text('BEAT FX'), findsOneWidget);
    expect(find.text('LEVEL'), findsOneWidget);
    expect(find.byType(MixarSlider), findsOneWidget);
    // The unit always carries an effect: no `Off` entry, Echo by default.
    expect(find.text('Echo'), findsOneWidget);
    expect(find.text('Off'), findsNothing);
    expect(find.byIcon(LucideIcons.chevronLeft), findsOneWidget);
    expect(find.byIcon(LucideIcons.chevronRight), findsOneWidget);
    // Before the first snapshot the readout matches the engine's echo default
    // (1/2 beat), not the first table entry.
    expect(find.text('1/2'), findsOneWidget);
    expect(
      find.byIcon(LucideIcons.power),
      findsNothing,
      reason: 'the arm toggle is the label, not a power glyph',
    );
    // One status-strip line, not a stacked panel.
    expect(tester.getSize(find.byType(FxBar)).height, FxBar.height);
  });

  testWidgets('right-packs the whole strip', (tester) async {
    // Wide strip: the content must sit at the right edge, not the left one.
    await pumpBar(tester, fxSlots: [echoSlot()], width: 900);

    final strip = tester.getRect(find.byType(FxBar));
    final select = tester.getRect(
      find.ancestor(
        of: find.byIcon(LucideIcons.chevronDown),
        matching: find.byType(MixerButton),
      ),
    );
    final beat = tester.getRect(find.byIcon(LucideIcons.chevronLeft));
    final level = tester.getRect(find.text('LEVEL'));
    final slider = tester.getRect(find.byType(MixarSlider));
    final arm = tester.getRect(
      find.ancestor(
        of: find.text('BEAT FX'),
        matching: find.byType(MixerButton),
      ),
    );
    expect(select.left, lessThan(beat.left));
    expect(beat.left, lessThan(level.left));
    expect(level.left, lessThan(slider.left));
    expect(slider.left, lessThan(arm.left));
    // Everything hugs the right edge; the left side stays empty.
    expect(arm.right, greaterThan(strip.right - 16));
    expect(select.left, greaterThan(strip.left + 100));
    // And it all sits on the same status-strip line.
    final centers = [select, level, slider, arm].map((r) => r.center.dy);
    for (final dy in centers) {
      expect((dy - centers.first).abs(), lessThan(4), reason: 'one line');
    }
  });

  testWidgets('echo steps its beat division with ◀ ▶', (tester) async {
    final (:container, :engine) = await pumpBar(
      tester,
      fxSlots: [echoSlot(enabled: true)],
    );
    expect(find.text('Echo'), findsOneWidget);
    expect(find.text('1/2'), findsOneWidget);

    await tester.tap(find.byIcon(LucideIcons.chevronRight));
    await tester.pumpAndSettle();
    final up = engine.fxUnitParams.single;
    expect(up.$1, 0, reason: 'slot');
    expect(up.$2, 0, reason: 'division is param 0');
    expect(up.$3, closeTo(4 / 6, 1e-9), reason: '1/2 → 3/4');

    await tester.tap(find.byIcon(LucideIcons.chevronLeft));
    await tester.pumpAndSettle();
    // The mirror stepped with the first tap, so down returns to 1/2.
    expect(
      engine.fxUnitParams.last.$3,
      closeTo(3 / 6, 1e-9),
      reason: '3/4 → 1/2',
    );

    // The engine echo shows the new division: the readout follows it.
    container
        .read(engineUiProvider.notifier)
        .apply(
          EngineEvt(
            kind: EngineEvtKind.updated,
            deckId: 0,
            fx: [echoSlot(enabled: true, divisionNorm: 1)],
            fxRouted: true,
          ),
        );
    await tester.pumpAndSettle();
    expect(find.text('2/1'), findsOneWidget);

    // Top of the table: ▶ has nowhere to go.
    await tester.tap(find.byIcon(LucideIcons.chevronRight));
    await tester.pumpAndSettle();
    expect(engine.fxUnitParams, hasLength(2));
  });

  testWidgets('reverb has no beat stepper', (tester) async {
    await pumpBar(tester, fxSlots: [reverbSlot()]);

    expect(find.text('Reverb'), findsOneWidget);
    expect(find.byIcon(LucideIcons.chevronLeft), findsNothing);
    expect(find.byIcon(LucideIcons.chevronRight), findsNothing);
  });

  testWidgets('the BEAT FX label arms the unit and flips its semantics', (
    tester,
  ) async {
    final handle = tester.ensureSemantics();
    try {
      final (:container, :engine) = await pumpBar(
        tester,
        fxSlots: [echoSlot()],
      );
      expect(find.bySemanticsLabel(RegExp('FX off')), findsOneWidget);

      await tester.tap(find.text('BEAT FX'));
      await tester.pumpAndSettle();
      expect(engine.fxUnitSlotEnabled, [(0, true)]);

      container
          .read(engineUiProvider.notifier)
          .apply(
            EngineEvt(
              kind: EngineEvtKind.updated,
              deckId: 0,
              fx: [echoSlot(enabled: true)],
              fxRouted: true,
            ),
          );
      await tester.pumpAndSettle();
      expect(find.bySemanticsLabel(RegExp('FX on')), findsOneWidget);
      expect(find.bySemanticsLabel(RegExp('FX off')), findsNothing);
    } finally {
      handle.dispose();
    }
  });

  testWidgets('the FX SELECT menu lists the effects and publishes unarmed', (
    tester,
  ) async {
    final (:container, :engine) = await pumpBar(tester);

    await openEffectMenu(tester);
    // No `Off` entry: the unit always carries an effect.
    expect(find.text('Off'), findsNothing);
    expect(find.text('Echo'), findsWidgets);
    await tester.tap(find.text('Reverb'));
    await tester.pumpAndSettle();

    expect(engine.fxUnitSlots, [(0, FxKind.reverb)]);
    expect(engine.fxUnitSlotEnabled, isEmpty, reason: 'selection must not arm');
    expect(container.read(fxSlotsProvider), isEmpty, reason: 'no evt yet');

    // The engine echoes the selection back; reverb has no beat time to step.
    container
        .read(engineUiProvider.notifier)
        .apply(
          EngineEvt(
            kind: EngineEvtKind.updated,
            deckId: 0,
            fx: [reverbSlot()],
            fxRouted: true,
          ),
        );
    await tester.pumpAndSettle();
    expect(find.text('Reverb'), findsOneWidget);
    expect(find.byIcon(LucideIcons.chevronLeft), findsNothing);
  });

  testWidgets('LEVEL drives wet/dry', (tester) async {
    final harness = await pumpBar(tester, fxSlots: [echoSlot()]);
    final engine = harness.engine;

    // Loaded but unarmed: LEVEL stays pre-settable, as on hardware.
    await tester.drag(find.byType(MixarSlider), const Offset(40, 0));
    await tester.pumpAndSettle();
    expect(engine.fxUnitWets, hasLength(1));
    expect(engine.fxUnitWets.single.$1, 0, reason: 'slot');
    expect(engine.fxUnitWets.single.$2, greaterThan(0.5));
  });

  testWidgets('a stopped engine leaves the strip inert', (tester) async {
    final (:container, :engine) = await pumpBar(
      tester,
      running: false,
      fxSlots: [echoSlot()],
    );

    expect(container.read(engineRunningProvider), isFalse);
    await tester.tap(find.text('BEAT FX'));
    await tester.pumpAndSettle();
    expect(engine.fxUnitSlotEnabled, isEmpty);
    await tester.drag(find.byType(MixarSlider), const Offset(40, 0));
    await tester.pumpAndSettle();
    expect(engine.fxUnitWets, isEmpty);

    await openEffectMenu(tester);
    expect(find.text('Reverb'), findsNothing, reason: 'menu must be disabled');
    expect(engine.fxUnitSlots, isEmpty);
  });

  testWidgets('the strip still fits a narrow window', (tester) async {
    // Echo with its stepper is the widest the strip gets.
    await pumpBar(tester, fxSlots: [echoSlot(enabled: true)], width: 320);

    expect(tester.takeException(), isNull, reason: 'no layout overflow');
    expect(find.text('1/2'), findsOneWidget);
    expect(find.text('LEVEL'), findsOneWidget);
  });

  testWidgets('two quick presses arm then disarm before the engine echoes', (
    tester,
  ) async {
    final (:container, :engine) = await pumpBar(tester, fxSlots: [echoSlot()]);

    await tester.tap(find.text('BEAT FX'));
    await tester.pumpAndSettle();
    // Optimistic mirror flipped, so the second press reads "on" and sends off.
    expect(container.read(fxSlotsProvider).single.enabled, isTrue);
    await tester.tap(find.text('BEAT FX'));
    await tester.pumpAndSettle();
    expect(container.read(fxSlotsProvider).single.enabled, isFalse);
    expect(engine.fxUnitSlotEnabled, [(0, true), (0, false)]);
  });

  testWidgets('two quick division taps step twice before the engine echoes', (
    tester,
  ) async {
    final (:container, :engine) = await pumpBar(tester, fxSlots: [echoSlot()]);

    expect(find.text('1/2'), findsOneWidget);
    await tester.tap(find.byIcon(LucideIcons.chevronRight));
    await tester.pumpAndSettle();
    // The mirror stepped even though no DeckUpdated echoed.
    expect(find.text('3/4'), findsOneWidget);
    await tester.tap(find.byIcon(LucideIcons.chevronRight));
    await tester.pumpAndSettle();
    expect(find.text('1/1'), findsOneWidget);
    expect(
      container.read(fxSlotsProvider).single.params.first,
      closeTo(fxDivisionNorm(5), 1e-6),
      reason: 'the mirror stores the knob as f32',
    );
    expect(engine.fxUnitParams, [
      (0, 0, fxDivisionNorm(4)),
      (0, 0, fxDivisionNorm(5)),
    ]);
  });
}
