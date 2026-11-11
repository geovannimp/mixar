import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/src/rust/api/event_mirrors.dart' show SavedLoop;
import 'package:gui_flutter/widgets/mixer/waveform/loop_band_hit_test.dart';

void main() {
  test('waveformMsAtX maps the edges and middle', () {
    expect(waveformMsAtX(x: 0, width: 400, originMs: 1000, spanMs: 8000), 1000);
    expect(
      waveformMsAtX(x: 200, width: 400, originMs: 1000, spanMs: 8000),
      5000,
    );
    expect(
      waveformMsAtX(x: 400, width: 400, originMs: 1000, spanMs: 8000),
      9000,
    );
  });

  test('waveformMsAtX clamps out-of-range x', () {
    expect(
      waveformMsAtX(x: -50, width: 400, originMs: 1000, spanMs: 8000),
      1000,
    );
    expect(
      waveformMsAtX(x: 450, width: 400, originMs: 1000, spanMs: 8000),
      9000,
    );
    expect(waveformMsAtX(x: 10, width: 0, originMs: 1000, spanMs: 8000), 1000);
  });

  test('savedLoopAtX picks the tightest loop under the point', () {
    const loops = [
      SavedLoop(slot: 2, inMs: 0, outMs: 8000),
      SavedLoop(slot: 0, inMs: 1000, outMs: 3000),
      SavedLoop(slot: 1, inMs: 1000, outMs: 3000),
    ];

    final hit = savedLoopAtX(
      loops: loops,
      x: 20,
      width: 100,
      originMs: 0,
      spanMs: 10000,
    );

    expect(hit?.slot, 0);
  });

  test('savedLoopAtX returns null outside every loop', () {
    const loops = [SavedLoop(slot: 0, inMs: 1000, outMs: 3000)];

    expect(
      savedLoopAtX(loops: loops, x: 90, width: 100, originMs: 0, spanMs: 10000),
      isNull,
    );
  });
}
