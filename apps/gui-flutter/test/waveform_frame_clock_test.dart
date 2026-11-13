import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/widgets/mixer/waveform/waveform_frame_clock.dart';

void main() {
  test('a zero-length glide settles at the retarget instant', () {
    final glide = DriftGlide();
    glide.retarget(at: Duration.zero, target: 20, glide: Duration.zero);
    expect(glide.valueAt(Duration.zero), 20);
    expect(glide.pendingAt(Duration.zero), isFalse);
  });

  test('retargeting mid-glide keeps the interpolated progress', () {
    final glide = DriftGlide();
    glide.retarget(
      at: Duration.zero,
      target: 100,
      glide: const Duration(milliseconds: 1000),
    );
    expect(glide.valueAt(const Duration(milliseconds: 500)), closeTo(50, 1e-9));
    glide.retarget(
      at: const Duration(milliseconds: 500),
      target: 80,
      glide: const Duration(milliseconds: 100),
    );
    expect(glide.valueAt(const Duration(milliseconds: 500)), closeTo(50, 1e-9));
    expect(glide.valueAt(const Duration(milliseconds: 600)), closeTo(80, 1e-9));
  });

  testWidgets('the clock ref-counts attachments', (tester) async {
    final clock = WaveformFrameClock();
    addTearDown(() {
      // Leave no transient callback behind: detach, then dispose.
      while (clock.attachedCount > 0) {
        clock.detach();
      }
      clock.dispose();
    });
    expect(clock.attachedCount, 0);
    expect(clock.isRunning, isFalse);

    clock.attach();
    expect(clock.attachedCount, 1);
    expect(clock.isRunning, isTrue);

    clock.attach();
    expect(clock.attachedCount, 2);

    clock.detach();
    expect(clock.attachedCount, 1);
    expect(clock.isRunning, isTrue, reason: 'one lane still needs frames');

    clock.detach();
    expect(clock.attachedCount, 0);
    expect(clock.isRunning, isFalse);

    // A surplus detach must not underflow the count or restart the ticker.
    clock.detach();
    expect(clock.attachedCount, 0);
    expect(clock.isRunning, isFalse);
  });

  testWidgets('now keeps advancing across a stop and start', (tester) async {
    final clock = WaveformFrameClock();
    addTearDown(() {
      // Leave no transient callback behind: detach, then dispose.
      while (clock.attachedCount > 0) {
        clock.detach();
      }
      clock.dispose();
    });

    clock.attach();
    await tester.pump(const Duration(milliseconds: 16));
    await tester.pump(const Duration(milliseconds: 100));
    final first = clock.now;
    expect(first, greaterThan(Duration.zero));

    clock.detach();
    await tester.pump(const Duration(milliseconds: 100));
    expect(clock.now, first, reason: 'a stopped clock must not advance');

    clock.attach();
    await tester.pump(const Duration(milliseconds: 16));
    expect(clock.now, first, reason: 'a restart rebases instead of jumping');
    await tester.pump(const Duration(milliseconds: 50));
    expect(clock.now, greaterThan(first));

    // flutter_test checks for leftover transient callbacks before teardown, so
    // the clock has to be stopped inside the test body.
    clock.detach();
  });
}
