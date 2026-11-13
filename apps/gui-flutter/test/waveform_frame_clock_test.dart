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
}
