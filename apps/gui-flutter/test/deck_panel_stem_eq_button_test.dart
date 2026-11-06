import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/models/engine_ui.dart';
import 'package:gui_flutter/core/theme/deck_accent.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/providers/shell/controller_providers.dart';
import 'package:gui_flutter/widgets/mixer/deck_panel.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart' show Scaffold;

import 'support/fake_engine_transport.dart';
import 'support/mixar_material_app.dart';

/// Engine mirror seeded with the deck states the header button reacts to.
class _SeedEngineUi extends EngineUi {
  _SeedEngineUi(this.seed);

  final EngineUiSnapshot seed;

  @override
  EngineUiSnapshot build() => seed;
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  Future<FakeEngineTransport> pumpDeck(
    WidgetTester tester, {
    required EngineUiSnapshot seed,
  }) async {
    final engine = FakeEngineTransport();
    addTearDown(engine.dispose);
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          engineUiProvider.overrideWith(() => _SeedEngineUi(seed)),
          engineTransportProvider.overrideWith((ref) async => engine),
          appSettingsProvider.overrideWith((ref) async => defaultAppSettings()),
          audioDevicesProvider.overrideWith((ref) async => const []),
          samplerBanksProvider.overrideWith((ref) async => const []),
          controllerTransportProvider.overrideWith((ref) async => null),
        ],
        child: mixarTestMaterialApp(
          theme: MixarThemeData.dark(),
          home: const Scaffold(
            body: DeckPanel(deckId: 0, label: 'DECK A', accent: DeckAccent.a),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    return engine;
  }

  EngineUiSnapshot seed({
    required bool stemsAudible,
    bool stemEqEnabled = false,
  }) => EngineUiSnapshot(
    running: true,
    trackPaths: const {0: '/song.wav'},
    trackIds: const {0: 't1'},
    durationMs: const {0: 180000},
    stemsAudible: {0: stemsAudible},
    stemEqEnabled: {0: stemEqEnabled},
  );

  testWidgets('ST asks the engine for stem EQ once stems are audible', (
    tester,
  ) async {
    final engine = await pumpDeck(tester, seed: seed(stemsAudible: true));

    await tester.tap(find.text('ST'));
    await tester.pumpAndSettle();

    expect(engine.stemEqEnabled, [(0, true)]);
  });

  testWidgets('a lit ST turns stem EQ back off', (tester) async {
    final engine = await pumpDeck(
      tester,
      seed: seed(stemsAudible: true, stemEqEnabled: true),
    );

    await tester.tap(find.text('ST'));
    await tester.pumpAndSettle();

    expect(engine.stemEqEnabled, [(0, false)]);
  });

  testWidgets('ST stays inert while the deck is not playing stems', (
    tester,
  ) async {
    final engine = await pumpDeck(tester, seed: seed(stemsAudible: false));

    await tester.tap(find.text('ST'));
    await tester.pumpAndSettle();

    expect(engine.stemEqEnabled, isEmpty);
  });
}
