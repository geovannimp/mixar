import 'package:gui_flutter/src/rust/api/event_mirrors.dart' show SavedLoop;
import 'package:gui_flutter/widgets/mixer/deck_loop_panel.dart'
    show savedLoopAtPosition;

/// Maps a waveform x coordinate to track milliseconds.
///
/// Values outside `[0, width]` clamp to the visible edges. A non-positive
/// width or span collapses to [originMs].
int waveformMsAtX({
  required double x,
  required double width,
  required int originMs,
  required int spanMs,
}) {
  if (width <= 0 || spanMs <= 0) {
    return originMs;
  }
  final clamped = x.clamp(0.0, width);
  return originMs + (clamped / width * spanMs).round();
}

/// Returns the tightest saved loop under a waveform x coordinate.
SavedLoop? savedLoopAtX({
  required List<SavedLoop> loops,
  required double x,
  required double width,
  required int originMs,
  required int spanMs,
}) {
  return savedLoopAtPosition(
    loops,
    waveformMsAtX(x: x, width: width, originMs: originMs, spanMs: spanMs),
  );
}
