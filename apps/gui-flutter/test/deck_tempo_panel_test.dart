import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/mixer/deck_tempo_panel.dart';
import 'package:gui_flutter/mixer/fader_slider.dart';
import 'package:gui_flutter/mixer/tempo_format.dart';
import 'package:gui_flutter/settings/settings_defaults.dart'
    show defaultAppSettings;
import 'package:gui_flutter/settings/settings_providers.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:gui_flutter/src/rust/api/engine.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:material_ui/material_ui.dart';

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
                accent: FaderAccent.a,
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

  testWidgets('renders engine speed range and locked sync icon', (tester) async {
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

  testWidgets('master deck shows crown not lock; fader stays enabled; tap no-ops', (
    tester,
  ) async {
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
  });

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

  testWidgets('sync lock button sends tempo vs beat from shift', (tester) async {
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
}
