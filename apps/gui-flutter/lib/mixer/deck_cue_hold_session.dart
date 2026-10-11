import 'dart:async';

/// Orders cue-hold begin/end on one per-deck future.
///
/// End is ignored unless begin completed successfully. Dispose drains the
/// chain so an in-flight hold still gets its end cmd.
class DeckCueHoldSession {
  Future<void> _chain = Future<void>.value();
  var _began = false;
  var _beginQueued = false;
  var _endQueued = false;

  /// Enqueue [begin]; mark hold active only after it completes.
  ///
  /// Ignores re-entrant begins while a begin is queued or the hold is active.
  void begin(Future<void> Function() begin) {
    if (_began || _beginQueued) {
      return;
    }
    _beginQueued = true;
    final step = _chain.then((_) async {
      try {
        await begin();
        _began = true;
      } finally {
        _beginQueued = false;
      }
    });
    _chain = step.catchError((Object _) {
      _began = false;
      _beginQueued = false;
    });
  }

  /// Enqueue [end] only after a successful begin; otherwise no-op.
  ///
  /// Ignores a second end while one is already queued (child `onEndHold` +
  /// parent `disposeEnd` during unmount). Engine end failures keep `_began`
  /// true so a later [end]/[disposeEnd] can retry.
  void end(Future<void> Function() end) {
    // Fire-and-forget: callers use [disposeEnd] when they need to await.
    unawaited(_enqueueEnd(end, swallowErrors: true) ?? Future<void>.value());
  }

  /// Schedule end (if hold began) and wait for the session to drain.
  ///
  /// Unlike [end], errors from [end] propagate to the caller so dispose can
  /// observe a failed engine endCueHold. The session remains usable afterward:
  /// `_chain` is reset, and `_began` stays true when end fails so a retry can
  /// still release the engine hold.
  ///
  /// If [end] already queued an end (e.g. child button dispose), this awaits
  /// that step instead of sending a second engine command.
  Future<void> disposeEnd(Future<void> Function() end) async {
    final step = _enqueueEnd(end, swallowErrors: false);
    if (step == null) {
      await _chain;
      return;
    }
    try {
      await step;
    } catch (_) {
      // Keep the session reusable: clear the rejected chain and any stale
      // begin-queued latch. `_began` stays true so a later end can retry.
      _beginQueued = false;
      _endQueued = false;
      _chain = Future<void>.value();
      rethrow;
    }
  }

  /// Wait until queued begin/end steps settle (tests / callers).
  Future<void> drain() => _chain;

  /// Returns the scheduled step, or null when an end is already queued / no
  /// hold to end.
  Future<void>? _enqueueEnd(
    Future<void> Function() end, {
    required bool swallowErrors,
  }) {
    if (_endQueued) {
      return null;
    }
    // Allow end while begin is still in-flight; ignore when nothing to end.
    if (!_began && !_beginQueued) {
      return null;
    }
    _endQueued = true;
    final step = _chain.then((_) async {
      try {
        if (!_began) {
          return;
        }
        await end();
        _began = false;
      } finally {
        _endQueued = false;
      }
    });
    if (!swallowErrors) {
      _chain = step;
      return step;
    }
    return _chain = step.catchError((Object _) {
      // Keep `_began` true (end did not finish) and clear stale latches so
      // a later end/disposeEnd can retry. Caller UI already toasted via
      // `_engineCmd` when that wrapper is used.
      _beginQueued = false;
      _endQueued = false;
    });
  }
}
