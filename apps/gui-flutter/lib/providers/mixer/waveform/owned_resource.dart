import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/utils/owned_resource.dart';

export 'package:gui_flutter/core/utils/owned_resource.dart';

/// Generation and ownership for notifiers that publish an [OwnedResource] built
/// on an async path (an encoded texture, a recorded strip).
///
/// One implementation so the waveform strip and the overview cannot drift:
///
/// - a rebuild keeps the resource already on screen ([owned]) until its
///   replacement publishes, because blanking a lane for the frame(s) an async
///   build takes reads as a glitch;
/// - async work checks [isCurrent] before publishing, so stale work is
///   discarded instead of overwriting a newer build;
/// - a replaced resource is dropped only after the frame that may still paint
///   or sample it ([dropAfterFrame]).
mixin OwnedResourceNotifier<T extends OwnedResource> on Notifier<T?> {
  T? _owned;
  var _gen = 0;

  /// The resource currently published, which this build keeps on screen.
  T? get owned => _owned;

  /// Supersedes any work in flight and returns the new generation.
  int supersede() => ++_gen;

  /// Whether [generation] is still the newest build.
  bool isCurrent(int generation) => generation == _gen;

  /// Publishes [resource], dropping the one it replaces after the current
  /// frame.
  void publish(T resource) {
    final replaced = _owned;
    _owned = resource;
    state = resource;
    dropAfterFrame(replaced);
  }

  /// Drops [resource] if it is still the owned one.
  void releaseIfOwned(T? resource) {
    if (resource == null || !identical(_owned, resource)) {
      return;
    }
    _owned = null;
    dropAfterFrame(resource);
  }

  /// Releases whatever this notifier still owns (`ref.onDispose`).
  void releaseOwned() => releaseIfOwned(_owned);

  /// Drops [resource] once the frame that may still paint or sample it is gone.
  void dropAfterFrame(T? resource) {
    if (resource == null) {
      return;
    }
    WidgetsBinding.instance.addPostFrameCallback((_) => resource.dispose());
  }
}
