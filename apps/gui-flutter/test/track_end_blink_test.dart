import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/providers/mixer/track_end_warning.dart';
import 'package:gui_flutter/widgets/mixer/track_end_blink.dart';

/// Mutable inputs a test can move mid-run (a tempo change, reduce-motion).
class _Knobs {
  Duration period = const Duration(milliseconds: 4000);
  bool disableAnimations = false;
}

void main() {
  late _Knobs knobs;

  setUp(() {
    knobs = _Knobs();
  });

  ProviderContainer containerFor({required bool active}) {
    final container = ProviderContainer(
      overrides: [
        deckTrackEndWarningProvider.overrideWith((ref, deckId) => active),
        // Reads the live knob so a retime can be pushed through `invalidate`.
        deckEndWarningBlinkPeriodProvider.overrideWith(
          (ref, deckId) => knobs.period,
        ),
      ],
    );
    addTearDown(container.dispose);
    return container;
  }

  Widget harness(ProviderContainer container, {double minOpacity = 0.55}) {
    return UncontrolledProviderScope(
      container: container,
      child: MediaQuery(
        data: MediaQueryData(disableAnimations: knobs.disableAnimations),
        child: Directionality(
          textDirection: TextDirection.ltr,
          child: TrackEndBlink(
            deckId: 0,
            minOpacity: minOpacity,
            child: const SizedBox.square(dimension: 10),
          ),
        ),
      ),
    );
  }

  double opacity(WidgetTester tester) =>
      tester.widget<FadeTransition>(find.byType(FadeTransition)).opacity.value;

  testWidgets('blinks once per period, quickly, then stays opaque', (
    tester,
  ) async {
    await tester.pumpWidget(harness(containerFor(active: true)));

    expect(opacity(tester), 1.0);
    // 150 ms down, 150 ms back; the rest of the bar holds full opacity.
    await tester.pump(const Duration(milliseconds: 150));
    expect(opacity(tester), closeTo(0.55, 1e-6));
    await tester.pump(const Duration(milliseconds: 150));
    expect(opacity(tester), closeTo(1.0, 1e-6));
    await tester.pump(const Duration(milliseconds: 2000));
    expect(opacity(tester), closeTo(1.0, 1e-6));

    // The next period brings the blink back.
    await tester.pump(const Duration(milliseconds: 1700));
    expect(opacity(tester), closeTo(1.0, 1e-6));
    await tester.pump(const Duration(milliseconds: 150));
    expect(opacity(tester), closeTo(0.55, 1e-6));
  });

  testWidgets('a faster tempo blinks more often', (tester) async {
    knobs.period = const Duration(milliseconds: 2000);
    await tester.pumpWidget(harness(containerFor(active: true)));

    await tester.pump(const Duration(milliseconds: 150));
    expect(opacity(tester), closeTo(0.55, 1e-6));
    await tester.pump(const Duration(milliseconds: 150));
    expect(opacity(tester), closeTo(1.0, 1e-6));

    // The second blink arrives after 2 s, not after 4 s.
    await tester.pump(const Duration(milliseconds: 1700));
    await tester.pump(const Duration(milliseconds: 150));
    expect(opacity(tester), closeTo(0.55, 1e-6));
  });

  testWidgets('a tempo change retimes the running blink', (tester) async {
    final container = containerFor(active: true);
    await tester.pumpWidget(harness(container));
    await tester.pump(const Duration(milliseconds: 300));
    expect(opacity(tester), closeTo(1.0, 1e-6));

    // Tempo doubles mid-warning: the restart below is the retime, and the
    // next blink must arrive on the new 2 s period, not the old 4 s one.
    knobs.period = const Duration(milliseconds: 2000);
    container.invalidate(deckEndWarningBlinkPeriodProvider(0));
    await tester.pump();
    // Let the restarted ticker run its dip and settle again.
    await tester.pump(const Duration(milliseconds: 300));
    await tester.pump(const Duration(milliseconds: 300));
    expect(opacity(tester), closeTo(1.0, 1e-6));

    // ~2 s after the restart a blink must be in progress; on the old 4 s
    // period the lane would still be fully opaque here.
    await tester.pump(const Duration(milliseconds: 1200));
    expect(opacity(tester), 1.0);
    await tester.pump(const Duration(milliseconds: 300));
    expect(
      opacity(tester),
      lessThan(1.0),
      reason: 'the retimed period is in force for later blinks',
    );
  });

  testWidgets('stays fully opaque outside the warning window', (tester) async {
    await tester.pumpWidget(harness(containerFor(active: false)));

    for (var frame = 0; frame < 12; frame++) {
      await tester.pump(const Duration(milliseconds: 150));
      expect(opacity(tester), 1.0);
    }
  });

  testWidgets('reduce-motion holds a steady dim instead of blinking', (
    tester,
  ) async {
    knobs.disableAnimations = true;
    await tester.pumpWidget(harness(containerFor(active: true)));

    for (var frame = 0; frame < 4; frame++) {
      await tester.pump(const Duration(milliseconds: 150));
      expect(opacity(tester), closeTo(0.55, 1e-6));
    }
  });
}
