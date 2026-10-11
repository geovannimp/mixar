/// Orders cue-hold begin/end on one per-deck future.
///
/// End is ignored unless begin completed successfully. Dispose drains the
/// chain so an in-flight hold still gets its end cmd.
class DeckCueHoldSession {
  Future<void> _chain = Future<void>.value();
  var _began = false;
  var _beginQueued = false;

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
  /// Engine end failures keep `_began` true so a later [end]/[disposeEnd]
  /// can retry, and reset `_chain` so the session stays usable.
  void end(Future<void> Function() end) {
    final step = _chain.then((_) async {
      if (!_began) {
        return;
      }
      await end();
      _began = false;
    });
    _chain = step.catchError((Object _) {
      // Keep `_began` true (end did not finish) and clear any stale latch so
      // a later end/disposeEnd can retry. Caller UI already toasted via
      // `_engineCmd` when that wrapper is used.
      _beginQueued = false;
    });
  }

  /// Schedule end (if hold began) and wait for the session to drain.
  ///
  /// Unlike [end], errors from [end] propagate to the caller so dispose can
  /// observe a failed engine endCueHold. The session remains usable afterward:
  /// `_chain` is reset, and `_began` stays true when end fails so a retry can
  /// still release the engine hold.
  Future<void> disposeEnd(Future<void> Function() end) async {
    final step = _chain.then((_) async {
      if (!_began) {
        return;
      }
      await end();
      _began = false;
    });
    _chain = step;
    try {
      await _chain;
    } catch (_) {
      // Keep the session reusable: clear the rejected chain and any stale
      // begin-queued latch. `_began` stays true so a later end can retry.
      _beginQueued = false;
      _chain = Future<void>.value();
      rethrow;
    }
  }

  /// Wait until queued begin/end steps settle (tests / callers).
  Future<void> drain() => _chain;
}
