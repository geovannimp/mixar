import 'package:flutter/foundation.dart';
import 'package:flutter/scheduler.dart';

/// One frame-based time base shared by every scrolling lane.
///
/// Flutter hands each [Ticker] `timeStamp - its own start time`
/// (`scheduler/ticker.dart`, `_tick`), so two lanes driven by their own
/// controllers measure elapsed from different baselines. One shared ticker
/// gives every lane the same `now`, so nobody has to reason about per-lane
/// start times.
///
/// It never rebuilds a widget: listeners exist to mark the paint side dirty.
/// It ticks only while at least one lane is attached, and [now] stays monotonic
/// across stops because elapsed restarts at zero on each `start`.
class WaveformFrameClock extends ChangeNotifier {
  late final Ticker _ticker = Ticker(_onTick);
  int _attached = 0;
  Duration _origin = Duration.zero;
  Duration _now = Duration.zero;

  /// Monotonic time of the most recent tick (zero before the first one).
  Duration get now => _now;

  void attach() {
    if (_attached++ == 0) {
      _origin = _now;
      _ticker.start();
    }
  }

  void detach() {
    if (_attached == 0) {
      return;
    }
    if (--_attached == 0) {
      _ticker.stop();
    }
  }

  void _onTick(Duration elapsed) {
    _now = _origin + elapsed;
    notifyListeners();
  }
}

/// The process-wide clock every lane paints against.
final WaveformFrameClock waveformFrameClock = WaveformFrameClock();

/// A linear, time-interpolated offset on top of the playback clock.
///
/// The value is a function of time, never a value switched at a poll boundary:
/// retargeting reads the *current interpolated* value, so an unfinished
/// correction keeps its progress and the strip cannot take the instantaneous
/// step this design exists to avoid.
class DriftGlide {
  double _from = 0;
  double _to = 0;
  Duration _fromAt = Duration.zero;
  Duration _toAt = Duration.zero;
  bool _active = false;

  /// Offset to apply at [now].
  double valueAt(Duration now) {
    if (!_active) {
      return _to;
    }
    if (now <= _fromAt) {
      return _from;
    }
    if (now >= _toAt) {
      return _to;
    }
    final total = (_toAt - _fromAt).inMicroseconds;
    if (total <= 0) {
      return _to;
    }
    final k = (now - _fromAt).inMicroseconds / total;
    return _from + (_to - _from) * k;
  }

  /// Whether an interpolation is still running at [now].
  bool pendingAt(Duration now) => _active && now < _toAt;

  /// Glides from the current interpolated value toward [target].
  ///
  /// A non-positive [glide] settles immediately: with `_toAt == _fromAt` the
  /// "not started" branch in [valueAt] would win and report the old value while
  /// [pendingAt] already claimed the glide was finished.
  void retarget({
    required Duration at,
    required double target,
    required Duration glide,
  }) {
    if (glide <= Duration.zero) {
      _from = target;
      _to = target;
      _fromAt = at;
      _toAt = at;
      _active = false;
      return;
    }
    _from = valueAt(at);
    _fromAt = at;
    _to = target;
    _toAt = at + glide;
    _active = true;
  }

  /// Stops interpolating and holds the value reached at [at].
  void freezeAt(Duration at) {
    _to = valueAt(at);
    _from = _to;
    _fromAt = at;
    _toAt = at;
    _active = false;
  }

  /// Drops any offset: used when the position becomes authoritative.
  void reset() {
    _from = 0;
    _to = 0;
    _fromAt = Duration.zero;
    _toAt = Duration.zero;
    _active = false;
  }
}
