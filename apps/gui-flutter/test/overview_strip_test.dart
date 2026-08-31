import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/mixer/engine_ui.dart';
import 'package:gui_flutter/mixer/waveform/overview_strip.dart';
import 'package:gui_flutter/mixer/waveform/peaks.dart';
import 'package:gui_flutter/mixer/waveform/waveform_providers.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:gui_flutter/src/rust/api/engine.dart';
import 'package:material_ui/material_ui.dart';

class _SeededEngineUi extends EngineUi {
  @override
  EngineUiSnapshot build() => applyEngineEvt(
    EngineUiSnapshot.empty,
    const EngineEvt(
      kind: EngineEvtKind.updated,
      deckId: 0,
      track: 'T',
      trackId: 't1',
      durationMs: 100000,
    ),
  );
}

class _MidPlayheads extends DeckPlayheads {
  @override
  Map<int, int> build() => const {0: 50000};
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  testWidgets(
    'overview dims left of playhead and has no viewport window tint',
    (tester) async {
      final theme = FTheme.neutral.dark.desktop;
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            engineUiProvider.overrideWith(_SeededEngineUi.new),
            deckPlayheadsProvider.overrideWith(_MidPlayheads.new),
            waveformOverviewProvider.overrideWith(
              (ref, id) async => const [
                SpectralPeak(low: 1, mid: 1, high: 1),
                SpectralPeak(low: 1, mid: 1, high: 1),
              ],
            ),
            beatGridFetchProvider.overrideWith((ref, id) async => null),
          ],
          child: MaterialApp(
            theme: materialUiThemeFromForui(theme),
            builder: (context, child) => MaterialUiCompatibilityBridge(
              // ignore: deprecated_member_use
              child: FTheme(data: theme, child: child!),
            ),
            home: const Scaffold(
              body: SizedBox(
                width: 200,
                height: 36,
                child: OverviewStrip(deckId: 0, height: 36),
              ),
            ),
          ),
        ),
      );
      await tester.pump();
      await tester.pump();

      final dim = find.byWidgetPredicate((w) {
        if (w is! ColoredBox) {
          return false;
        }
        final c = w.color;
        return (c.a - 0.4).abs() < 0.001 && c.r == 0 && c.g == 0 && c.b == 0;
      });
      expect(dim, findsOneWidget);

      final positioned = tester.widget<Positioned>(
        find.ancestor(of: dim, matching: find.byType(Positioned)).first,
      );
      expect(positioned.width, closeTo(100, 0.5));

      final windowTint = find.byWidgetPredicate((w) {
        if (w is! ColoredBox) {
          return false;
        }
        return (w.color.a - 0.12).abs() < 0.001;
      });
      expect(windowTint, findsNothing);
    },
  );
}
