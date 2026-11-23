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

void _cancelSilkyState(State? state) {
  if (state == null) return;
  try {
    // silky_scroll 2.6.4 keeps its cancellable state on the private State.
    // ignore: avoid_dynamic_calls
    (state as dynamic).silkyScrollState.cancelSilkyScroll();
    // The package's internal field is untyped to us, so a shape change
    // surfaces as NoSuchMethodError rather than a compile error.
    // ignore: avoid_catching_errors
  } on NoSuchMethodError {
    // Upstream changed the internal state shape; let the glide finish.
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
