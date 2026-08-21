import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/deck_tempo_panel.dart';
import 'package:gui_flutter/mixer/fader_slider.dart';
import 'package:gui_flutter/mixer/tempo_format.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:gui_flutter/src/rust/api/engine.dart';
import 'package:material_ui/material_ui.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  Future<void> pumpPanel(
    WidgetTester tester, {
    double speed = 0.5,
    double tempoRange = kDefaultTempoRange,
    SyncMode syncMode = SyncMode.off,
    bool isMaster = false,
    List<double> tempoRangeSteps = kTempoRangeSteps,
    ValueChanged<double>? onSpeedChange,
    ValueChanged<double>? onTempoRangeChange,
    ValueChanged<bool>? onToggleSync,
    VoidCallback? onSetMaster,
  }) async {
    final theme = FTheme.neutral.dark.desktop;
    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromForui(theme),
        builder: (context, child) => MaterialUiCompatibilityBridge(
          // ignore: deprecated_member_use
          child: FTheme(data: theme, child: child!),
        ),
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
              tempoRangeSteps: tempoRangeSteps,
              onSpeedChange: onSpeedChange ?? (_) {},
              onTempoRangeChange: onTempoRangeChange ?? (_) {},
              onToggleSync: onToggleSync ?? (_) {},
              onSetMaster: onSetMaster ?? () {},
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  testWidgets('renders engine speed range sync and master labels', (
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
    expect(find.text('S'), findsOneWidget);
    expect(find.text('Set master'), findsOneWidget);
    expect(
      tester.widget<FaderSlider>(find.byType(FaderSlider)).disabled,
      isTrue,
    );
  });

  testWidgets('master deck shows M and Master; fader stays enabled', (
    tester,
  ) async {
    await pumpPanel(tester, isMaster: true);

    expect(find.text('M'), findsOneWidget);
    expect(find.text('Master'), findsOneWidget);
    expect(find.text('Sync'), findsNothing);
    expect(
      tester.widget<FaderSlider>(find.byType(FaderSlider)).disabled,
      isFalse,
    );
  });

  testWidgets('range tap reports next step without local display change', (
    tester,
  ) async {
    final ranges = <double>[];
    await pumpPanel(tester, tempoRange: 0.06, onTempoRangeChange: ranges.add);

    await tester.tap(find.text('±6%'));
    await tester.pumpAndSettle();

    expect(ranges, [0.10]);
    expect(find.text('±6%'), findsOneWidget);
    expect(find.text('±10%'), findsNothing);
  });

  testWidgets('sync chip sends tempo vs beat from shift', (tester) async {
    final beats = <bool>[];
    await pumpPanel(tester, onToggleSync: beats.add);

    await tester.tap(find.text('Sync'));
    await tester.pumpAndSettle();
    expect(beats, [false]);
    expect(find.text('Sync'), findsOneWidget);

    await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
    await tester.tap(find.text('Sync'));
    await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
    await tester.pumpAndSettle();
    expect(beats, [false, true]);
  });

  testWidgets('set master fires only when this deck is not master', (
    tester,
  ) async {
    var taps = 0;
    await pumpPanel(tester, onSetMaster: () => taps++);
    await tester.tap(find.text('Set master'));
    await tester.pumpAndSettle();
    expect(taps, 1);

    await pumpPanel(tester, isMaster: true, onSetMaster: () => taps++);
    await tester.tap(find.text('Master'));
    await tester.pumpAndSettle();
    expect(taps, 1);
  });
}
