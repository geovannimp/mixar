import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/misc.dart' show NotifierProviderFamily;
import 'package:gui_flutter/core/utils/waveform_layout.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/providers/mixer/waveform/waveform_zoom.dart';

/// Source-time window a lane's strip covers, playhead centered and clamped to
/// the track.
typedef WaveformWindow = ({int originMs, int spanMs, int visibleMs});

/// Tracks the playhead without re-fetching peaks every tick: the window is
/// recentred only when the visible span changes or the playhead nears an edge
/// ([l1NeedsRefresh]).
class WaveformWindowNotifier extends Notifier<WaveformWindow> {
  new(this.deckId);

  final int deckId;

  WaveformWindow? _current;

  @override
  WaveformWindow build() {
    final durationMs = ref.watch(deckDurationMsProvider(deckId)) ?? 0;
    final visibleMs = visibleSourceMs(
      ref.watch(deckSpeedRatioProvider(deckId)),
      ref.watch(waveformZoomProvider),
    );
    final positionMs = ref.watch(deckPositionMsProvider(deckId)).toDouble();
    final previous = _current;
    if (durationMs <= 0) {
      const empty = (originMs: 0, spanMs: 0, visibleMs: 0);
      _current = empty;
      return empty;
    }
    if (previous != null &&
        previous.visibleMs == visibleMs &&
        // A shorter track can leave the kept window past its end; re-centre
        // instead of asking for peaks the track does not have.
        previous.originMs + previous.spanMs <= durationMs &&
        !l1NeedsRefresh(
          positionMs: positionMs,
          detailStartMs: previous.originMs,
          detailEndMs: previous.originMs + previous.spanMs,
          visibleMs: visibleMs,
          durationMs: durationMs,
        )) {
      // Same instance, so consumers are not rebuilt on every position tick.
      return previous;
    }
    final range = l1Range(
      positionMs: positionMs.round(),
      visibleMs: visibleMs,
      durationMs: durationMs,
    );
    final next = (
      originMs: range.startMs,
      spanMs: range.endMs - range.startMs,
      visibleMs: visibleMs,
    );
    _current = next;
    return next;
  }
}

final NotifierProviderFamily<WaveformWindowNotifier, WaveformWindow, int>
waveformWindowProvider = NotifierProvider.autoDispose
    .family<WaveformWindowNotifier, WaveformWindow, int>(
      WaveformWindowNotifier.new,
    );
