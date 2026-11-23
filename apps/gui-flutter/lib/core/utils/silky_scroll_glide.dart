import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:silky_scroll/silky_scroll.dart';

/// Stops a `SilkyScroll`'s in-flight wheel glide without moving the viewport.
///
/// silky_scroll 2.6.4 exposes no public cancellation API, so its private State
/// is reached through the public `silkyScrollState` field. That state's
/// `cancelSilkyScroll()` stops the ticker; without it, a glide can keep
/// jumping the controller after a keyboard jump, `ensureVisible`, or a
/// scrollbar drag takes over.
///
/// The key may sit on `SilkyScroll` itself or on one of the preset widgets,
/// which are stateless wrappers around it.
///
/// Keep the dependency on 2.6.x. If upstream removes the field, the error is
/// swallowed and the glide settles normally instead of crashing the scroll.
void cancelSilkyGlide(GlobalKey key) {
  final state = key.currentState ?? _silkyStateBelow(key.currentContext);
  _cancelSilkyState(state);
}

/// Cancels an in-flight `SilkyScroll` glide when the user takes over.
///
/// A press stops the ticker before a row drag, scrollbar-thumb grab, or deck
/// drop begins. A scroll drag notification covers the cases where the thumb
/// starts its own drag without a pointer-down reaching this subtree.
///
/// A click that starts no native scroll gets the overscrolled position back
/// from the platform physics after a short delay. A drag or hold keeps
/// ownership, so no ballistic is started underneath it.
class SilkyGlideHandoff extends StatefulWidget {
  const new({required this.scrollKey, required this.child, super.key});

  final GlobalKey scrollKey;
  final Widget child;

  @override
  State<SilkyGlideHandoff> createState() => _SilkyGlideHandoffState();
}

class _SilkyGlideHandoffState extends State<SilkyGlideHandoff> {
  Timer? _recoveryTimer;
  var _dragTakenOver = false;

  @override
  void dispose() {
    _recoveryTimer?.cancel();
    super.dispose();
  }

  void _cancelGlide() {
    cancelSilkyGlide(widget.scrollKey);
  }

  void _scheduleRecovery() {
    _recoveryTimer?.cancel();
    if (_dragTakenOver) return;
    _recoveryTimer = Timer(const Duration(milliseconds: 100), () {
      if (mounted && !_dragTakenOver) {
        recoverSilkyBounds(widget.scrollKey);
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    return NotificationListener<ScrollStartNotification>(
      onNotification: (notification) {
        if (notification.dragDetails != null) {
          _dragTakenOver = true;
          _recoveryTimer?.cancel();
          _cancelGlide();
        }
        return false;
      },
      child: Listener(
        onPointerDown: (_) {
          _dragTakenOver = false;
          _recoveryTimer?.cancel();
          _cancelGlide();
        },
        onPointerUp: (_) => _scheduleRecovery(),
        onPointerCancel: (_) => _scheduleRecovery(),
        child: widget.child,
      ),
    );
  }
}

/// Stops the nearest ancestor `SilkyScroll`'s wheel glide.
///
/// Use this before a driven scroll (for example `Scrollable.ensureVisible`)
/// starts from a widget that lives inside a converted list.
void cancelSilkyGlideFor(BuildContext context) {
  State? state;
  context.visitAncestorElements((element) {
    if (element is StatefulElement && element.widget is SilkyScroll) {
      state = element.state;
      return false;
    }
    return true;
  });
  _cancelSilkyState(state);
}

/// Lets the platform physics settle an overscrolled position.
///
/// Only acts while the position is idle: a drag, hold, or ballistic owns the
/// position otherwise, and replacing it would interrupt that gesture.
void recoverSilkyBounds(GlobalKey key) {
  final state = key.currentState ?? _silkyStateBelow(key.currentContext);
  if (state == null) return;
  try {
    // silky_scroll 2.6.4 keeps its state on the private State.
    final dynamic silkyState = (state as dynamic).silkyScrollState;
    // The client controller is exposed on that same internal state.
    final ScrollController? controller =
        // The client controller is only reachable dynamically.
        // ignore: avoid_dynamic_calls
        silkyState.clientController as ScrollController?;
    if (controller == null || !controller.hasClients) return;
    final position = controller.position;
    if (position is! ScrollPositionWithSingleContext) return;
    if (!position.outOfRange) return;
    // A drag, hold, or ballistic owns the position while this is not idle;
    // `isScrollingNotifier` is false for a hold, so inspect the activity.
    // ignore: invalid_use_of_protected_member, invalid_use_of_visible_for_testing_member
    final activity = position.activity;
    if (activity != null && activity is! IdleScrollActivity) return;
    position.goBallistic(0);
    // The package's internal field is untyped to us, so a shape change
    // surfaces as NoSuchMethodError rather than a compile error.
    // ignore: avoid_catching_errors
  } on NoSuchMethodError {
    // Upstream changed the internal state shape; leave the position alone.
    assert(() {
      debugPrint(
        'silky_scroll 2.6.4 bounds-recovery adapter failed; '
        'the glide is left to settle on its own.',
      );
      return true;
    }(), 'silky_scroll bounds recovery no longer matches 2.6.4');
  }
}

void _cancelSilkyState(State? state) {
  if (state == null) return;
  try {
    // silky_scroll 2.6.4 keeps its cancellable state on the private State.
    final dynamic silkyState = (state as dynamic).silkyScrollState;
    // The state is only reachable dynamically.
    // ignore: avoid_dynamic_calls
    silkyState.cancelSilkyScroll();
    // The package's internal field is untyped to us, so a shape change
    // surfaces as NoSuchMethodError rather than a compile error.
    // ignore: avoid_catching_errors
  } on NoSuchMethodError {
    // Upstream changed the internal state shape; let the glide finish.
    assert(() {
      debugPrint(
        'silky_scroll 2.6.4 cancellation adapter failed; '
        'the glide is left running.',
      );
      return true;
    }(), 'silky_scroll cancellation no longer matches 2.6.4');
  }
}

/// Finds the first [SilkyScroll] state below [context].
State? _silkyStateBelow(BuildContext? context) {
  if (context is! Element) return null;
  State? found;
  void visit(Element element) {
    if (found != null) return;
    if (element is StatefulElement && element.widget is SilkyScroll) {
      found = element.state;
      return;
    }
    element.visitChildElements(visit);
  }

  visit(context);
  return found;
}
