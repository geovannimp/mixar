import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/models/engine_ui.dart';
import 'package:gui_flutter/core/theme/deck_accent.dart';
import 'package:gui_flutter/core/utils/desktop.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/providers/shell/controller_providers.dart';
import 'package:gui_flutter/widgets/mixer/deck_cue_button.dart';
import 'package:gui_flutter/widgets/mixer/deck_panel.dart';
import 'package:gui_flutter/widgets/mixer/deck_performance_panel.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:material_ui/material_ui.dart' show Scaffold;

import 'support/fake_engine_transport.dart';
import 'support/mixar_material_app.dart';

class _RunningEngineUi extends EngineUi {
  @override
  EngineUiSnapshot build() =>
      const EngineUiSnapshot(running: true, trackPaths: {});
}

/// Tolerance for sub-pixel layout rounding.
const _kEpsilon = 0.5;

void main() {
  Future<void> pumpDeck(
    WidgetTester tester, {
    required Size size,
    int deckId = 0,
  }) async {
    debugOverrideDesktopWindow = false;
    addTearDown(() => debugOverrideDesktopWindow = null);
    tester.view.physicalSize = size;
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          engineUiProvider.overrideWith(_RunningEngineUi.new),
          engineTransportProvider.overrideWith(
            (ref) async => FakeEngineTransport(),
          ),
          appSettingsProvider.overrideWith((ref) async => defaultAppSettings()),
          audioDevicesProvider.overrideWith((ref) async => const []),
          samplerBanksProvider.overrideWith((ref) async => const []),
          controllerTransportProvider.overrideWith((ref) async => null),
        ],
        child: mixarTestMaterialApp(
          theme: MixarThemeData.dark(),
          home: Scaffold(
            body: DeckPanel(
              deckId: deckId,
              label: 'DECK A',
              accent: DeckAccent.a,
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  ({Rect panel, Rect cue, Rect play}) rects(WidgetTester tester) => (
    panel: tester.getRect(find.byType(DeckPerformancePanel)),
    cue: tester.getRect(find.byType(DeckCueButton)),
    play: tester.getRect(find.byIcon(LucideIcons.play)),
  );

  testWidgets('short deck stacks transport beside the panel at full height', (
    tester,
  ) async {
    await pumpDeck(tester, size: const Size(370, 316));

    final (:panel, :cue, :play) = rects(tester);

    // Beside the pads, not under them.
    expect(cue.left, greaterThanOrEqualTo(panel.right - _kEpsilon));
    // Top to bottom: the pair splits the pads area, no dead strip below.
    expect(cue.top, closeTo(panel.top, _kEpsilon));
    expect(cue.height, closeTo((panel.height - 8) / 2, _kEpsilon));
    // Only the width is capped.
    expect(cue.width, lessThanOrEqualTo(72));
    // Stacked: same column, play below cue. The play glyph carries a 1px
    // optical nudge, hence the looser tolerance.
    expect((cue.center.dx - play.center.dx).abs(), lessThan(1.5));
    expect(play.center.dy, greaterThan(cue.center.dy));
    // The row's height now belongs to the pads.
    expect(panel.height, greaterThan(120));
  });

  testWidgets('tall deck keeps the full-width transport row under the panel', (
    tester,
  ) async {
    await pumpDeck(tester, size: const Size(370, 560));

    final (:panel, :cue, :play) = rects(tester);

    // Side by side under the panel, spanning the deck width.
    expect(cue.top, greaterThanOrEqualTo(panel.bottom - _kEpsilon));
    expect(play.top, greaterThanOrEqualTo(panel.bottom - _kEpsilon));
    expect(cue.left, lessThan(panel.right));
    expect(play.right, greaterThan(panel.center.dx));
    // Half-width each, not squares.
    expect(cue.width, greaterThan(cue.height * 1.5));
  });
}
