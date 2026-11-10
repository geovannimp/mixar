import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/utils/waveform_layout.dart';

/// Source time on screen, shared by every lane so both decks stay aligned.
///
/// Scroll over a lane zooms around the fixed center playhead; double-click
/// resets to [kWaveformVisibleMs]. View state only: nothing is persisted and
/// no engine call is involved.
class WaveformZoom extends Notifier<int> {
  @override
  int build() => kWaveformVisibleMs;

  void zoomBy(double scrollDeltaY) {
    final next = zoomedVisibleMs(visibleMs: state, scrollDeltaY: scrollDeltaY);
    if (next != state) {
      state = next;
    }
  }

  void reset() {
    if (state != kWaveformVisibleMs) {
      state = kWaveformVisibleMs;
    }
  }
}

final waveformZoomProvider = NotifierProvider<WaveformZoom, int>(
  WaveformZoom.new,
);
