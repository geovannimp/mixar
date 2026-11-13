import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/models/engine_ui.dart';
import 'package:gui_flutter/core/models/waveform_peaks.dart';
import 'package:gui_flutter/core/theme/material_theme.dart';
import 'package:gui_flutter/core/utils/waveform_section_painter.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/providers/mixer/waveform/waveform_providers.dart';
import 'package:gui_flutter/widgets/mixer/waveform/overview_strip.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/bus_events.dart';
import 'support/mixar_material_app.dart';

class _SeededEngineUi extends EngineUi {
  @override
  EngineUiSnapshot build() => applyEngineEvent(
    EngineUiSnapshot.empty,
    deckUpdatedEvent(
      update: deckUpdated(
        trackPath: '/t.flac',
        trackId: 't1',
        durationMs: 100000,
      ),
    ),
  );
}

class _CuedEngineUi extends EngineUi {
  @override
  EngineUiSnapshot build() => applyEngineEvent(
    EngineUiSnapshot.empty,
    deckUpdatedEvent(
      update: deckUpdated(
        trackPath: '/t.flac',
        trackId: 't1',
        durationMs: 100000,
        cuePointMs: 25000,
      ),
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
      final theme = MixarThemeData.dark();
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
            theme: materialUiThemeFromMixar(theme),
            builder: mixarMaterialAppBuilder(theme),
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
        return (c.a - 0.6).abs() < 0.001 && c.r == 0 && c.g == 0 && c.b == 0;
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

  testWidgets('the main cue is flagged on the overview', (tester) async {
    final theme = MixarThemeData.dark();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          engineUiProvider.overrideWith(_CuedEngineUi.new),
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
          theme: materialUiThemeFromMixar(theme),
          builder: mixarMaterialAppBuilder(theme),
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

    final mainCue = [
      for (final section in _sections(tester))
        if (section.type == WaveformSectionType.mainCue) section,
    ];
    expect(mainCue, hasLength(1));
    expect(mainCue.single.startMs, 25000);
    expect(
      mainCue.single.color,
      theme.colors.cueOn,
      reason: 'the flag reuses the CUE button accent',
    );
    expect(
      mainCue.single.label,
      isNull,
      reason: 'the main cue carries an icon, not a hot-cue slot number',
    );
  });
}

/// Sections the overview overlay hands to its painter.
List<WaveformSection> _sections(WidgetTester tester) {
  for (final paint in tester.widgetList<CustomPaint>(
    find.byType(CustomPaint),
  )) {
    final painter = paint.painter;
    if (painter is WaveformSectionPainter) {
      return painter.sections;
    }
  }
  fail('the overview has no WaveformSectionPainter');
}
