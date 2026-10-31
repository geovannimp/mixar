import 'package:flutter/rendering.dart' show RenderParagraph;
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/models/bus_enums.dart';
import 'package:gui_flutter/core/theme/material_theme.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart'
    show defaultAppSettings;
import 'package:gui_flutter/core/utils/tempo_format.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/widgets/mixer/deck_tempo_panel.dart';
import 'package:gui_flutter/widgets/mixer/fader_slider.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:material_ui/material_ui.dart';
import 'package:gui_flutter/core/theme/deck_accent.dart';

import 'support/mixar_material_app.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  Future<void> pumpPanel(
    WidgetTester tester, {
    double speed = 0.5,
    double tempoRange = kDefaultTempoRange,
    SyncMode syncMode = SyncMode.off,
    bool isMaster = false,
    double? trackBpm,
    List<double> tempoRangeSteps = kTempoRangeSteps,
    ValueChanged<double>? onSpeedChange,
    ValueChanged<double>? onTempoRangeChange,
    ValueChanged<bool>? onToggleSync,
  }) async {
    final theme = MixarThemeData.dark();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          appSettingsProvider.overrideWith((ref) async => defaultAppSettings()),
        ],
        child: MaterialApp(
          theme: materialUiThemeFromMixar(theme),
          builder: mixarMaterialAppBuilder(theme),
          home: Scaffold(
            body: SizedBox(
              width: 140,
              height: 520,
              child: DeckTempoPanel(
                accent: DeckAccent.a,
                speed: speed,
                tempoRange: tempoRange,
                syncMode: syncMode,
                isMaster: isMaster,
                trackBpm: trackBpm,
                tempoRangeSteps: tempoRangeSteps,
                onSpeedChange: onSpeedChange ?? (_) {},
                onTempoRangeChange: onTempoRangeChange ?? (_) {},
                onToggleSync: onToggleSync ?? (_) {},
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  testWidgets('renders engine speed range and locked sync icon', (
    tester,
  ) async {
    await pumpPanel(
      tester,
      speed: 0.25,
      tempoRange: 0.08,
      syncMode: SyncMode.tempo,
    );

    expect(find.text(formatPitchPercent(0.25, 0.08)), findsOneWidget);
    expect(find.text(formatTempoRange(0.08)), findsOneWidget);
    expect(find.byIcon(LucideIcons.lock), findsOneWidget);
    expect(find.byIcon(LucideIcons.lockOpen), findsNothing);
    expect(find.text('Sync'), findsNothing);
    expect(find.text('S'), findsNothing);
    expect(find.text('Set master'), findsNothing);
    expect(find.text('Master'), findsNothing);
    expect(
      tester.widget<FaderSlider>(find.byType(FaderSlider)).disabled,
      isTrue,
    );
  });

  testWidgets(
    'master deck shows crown not lock; fader stays enabled; tap no-ops',
    (tester) async {
      final beats = <bool>[];
      await pumpPanel(tester, isMaster: true, onToggleSync: beats.add);

      expect(find.text('M'), findsNothing);
      expect(find.text('Master'), findsNothing);
      expect(find.text('Set master'), findsNothing);
      expect(find.text('Sync'), findsNothing);
      expect(find.byIcon(LucideIcons.crown), findsOneWidget);
      expect(find.byIcon(LucideIcons.lockOpen), findsNothing);
      expect(find.byIcon(LucideIcons.lock), findsNothing);
      expect(
        tester.widget<FaderSlider>(find.byType(FaderSlider)).disabled,
        isFalse,
      );

      await tester.tap(find.byIcon(LucideIcons.crown));
      await tester.pumpAndSettle();
      expect(beats, isEmpty);
    },
  );

  testWidgets('master deck BPM tap does not toggle sync', (tester) async {
    final beats = <bool>[];
    await pumpPanel(
      tester,
      isMaster: true,
      trackBpm: 128.0,
      onToggleSync: beats.add,
    );

    final bpmText = formatBpm(effectiveBpm(128.0, 0.5, kDefaultTempoRange));
    await tester.tap(find.text(bpmText));
    await tester.pumpAndSettle();
    expect(beats, isEmpty);
  });

  testWidgets('range tap reports next step without local display change', (
    tester,
  ) async {
    final ranges = <double>[];
    await pumpPanel(tester, onTempoRangeChange: ranges.add);

    await tester.tap(find.text('±6%'));
    await tester.pumpAndSettle();

    expect(ranges, [0.10]);
    expect(find.text('±6%'), findsOneWidget);
    expect(find.text('±10%'), findsNothing);
  });

  testWidgets('sync lock button sends tempo vs beat from shift', (
    tester,
  ) async {
    final beats = <bool>[];
    await pumpPanel(tester, onToggleSync: beats.add);

    expect(find.byIcon(LucideIcons.lockOpen), findsOneWidget);
    await tester.tap(find.byIcon(LucideIcons.lockOpen));
    await tester.pumpAndSettle();
    expect(beats, [false]);

    await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
    await tester.tap(find.byIcon(LucideIcons.lockOpen));
    await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
    await tester.pumpAndSettle();
    expect(beats, [false, true]);
  });

  testWidgets('BPM area of sync button also toggles sync', (tester) async {
    final beats = <bool>[];
    await pumpPanel(tester, trackBpm: 128.0, onToggleSync: beats.add);

    final bpmText = formatBpm(effectiveBpm(128.0, 0.5, kDefaultTempoRange));
    await tester.tap(find.text(bpmText));
    await tester.pumpAndSettle();
    expect(beats, [false]);

    await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
    await tester.tap(find.text(bpmText));
    await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
    await tester.pumpAndSettle();
    expect(beats, [false, true]);
  });

  testWidgets('narrow rail keeps every readout on one line', (tester) async {
    await pumpPanel(tester, trackBpm: 174, speed: 0, tempoRange: 0.25);

    for (final text in [
      formatBpm(effectiveBpm(174, 0, 0.25)),
      formatPitchPercent(0, 0.25),
      formatTempoRange(0.25),
    ]) {
      final finder = find.text(text);
      expect(finder, findsOneWidget);
      final paragraph = tester.renderObject<RenderParagraph>(finder);
      final painter = TextPainter(
        text: paragraph.text,
        textDirection: TextDirection.ltr,
      )..layout();
      expect(
        painter.width,
        lessThanOrEqualTo(paragraph.constraints.maxWidth),
        reason: '"$text" must fit the rail without wrapping',
      );
      painter.dispose();
    }
    expect(tester.takeException(), isNull);
  });
}
