import 'package:anchor_ui/anchor_ui.dart';
import 'package:flutter/widgets.dart';
import 'package:gui_flutter/providers/shell/mixar_overlay_controller.dart';

class MenuDialogController extends MixarOverlayController {
  MenuDialogController({required this.onOpen});

  final Future<void> Function() onOpen;
  NavigatorState? _dialogNavigator;
  Future<void>? _dialogFuture;
  var _open = false;
  var _disposed = false;

  @override
  bool get isShowing => _open;

  void attachNavigator(NavigatorState navigator) {
    _dialogNavigator = navigator;
  }

  @override
  void show() {
    // Block while a dialog route is still in flight / animating out so rapid
    // hide → show cannot stack a second route on the still-popping first one.
    if (_disposed || _open || _dialogFuture != null) return;
    _open = true;
    _notify();
    final Future<void> future;
    try {
      future = onOpen();
    } catch (_) {
      // Sync failure must not wedge `_open` forever with no route to hide.
      _open = false;
      _dialogNavigator = null;
      _dialogFuture = null;
      _notify();
      return;
    }
    _dialogFuture = future;
    future.whenComplete(() {
      if (!identical(_dialogFuture, future)) return;
      _dialogFuture = null;
      _dialogNavigator = null;
      if (_disposed) return;
      if (!_open) return;
      _open = false;
      _notify();
    });
  }

  @override
  void hide() {
    if (_disposed || !_open) return;
    final nav = _dialogNavigator;
    _open = false;
    _dialogNavigator = null;
    _notify();
    if (nav != null && nav.canPop()) {
      nav.pop();
    }
  }

  @override
  void toggle() => _open ? hide() : show();

  @override
  void dispose() {
    // Do not nav.pop() here (sync or post-frame): Wolt rebuilds
    // `modalTypeBuilder` and looks up MixarTheme on the deactivated host.
    // Prefer [hide] before tearing the host down; late whenComplete callbacks
    // are ignored via [_disposed].
    _disposed = true;
    _dialogNavigator = null;
    _open = false;
    _dialogFuture = null;
    super.dispose();
  }

  void _notify() {
    if (_disposed) return;
    notifyListeners();
  }
}

/// Primary-button outside tap dismisses (manual [Anchor] has no TapRegion dismiss).
class MixarOverlayDismissBackdrop extends StatelessWidget {
  const new({required this.onDismiss});

  final VoidCallback onDismiss;

  @override
  Widget build(BuildContext context) {
    return Listener(
      onPointerDown: (event) {
        if (event.buttons == 1) {
          onDismiss();
        }
      },
      behavior: HitTestBehavior.opaque,
      child: const SizedBox.expand(),
    );
  }
}
