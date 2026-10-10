/// Orders cue-hold begin/end on one per-deck future.
///
/// End is ignored unless begin completed successfully. Dispose drains the
/// chain so a in-flight hold still gets its end cmd.
class DeckCueHoldSession {
  Future<void> _chain = Future<void>.value();
  var _began = false;

  /// Enqueue [begin]; mark hold active only after it completes.
  void begin(Future<void> Function() begin) {
    final step = _chain.then((_) async {
      await begin();
      _began = true;
    });
    _chain = step.catchError((Object _) {
      _began = false;
    });
  }

  /// Enqueue [end] only after a successful begin; otherwise no-op.
  void end(Future<void> Function() end) {
    final step = _chain.then((_) async {
      if (!_began) {
        return;
      }
      _began = false;
      await end();
    });
    _chain = step.catchError((Object _) {});
  }

  /// Schedule end (if hold began) and wait for the session to drain.
  Future<void> disposeEnd(Future<void> Function() end) async {
    this.end(end);
    await _chain;
  }

  /// Wait until queued begin/end steps settle (tests / callers).
  Future<void> drain() => _chain;
}
