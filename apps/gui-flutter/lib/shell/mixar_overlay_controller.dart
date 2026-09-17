import 'package:anchor_ui/anchor_ui.dart';
import 'package:flutter/foundation.dart';

/// Thin wrap of [AnchorController] for Mixar popovers / button menus.
class MixarOverlayController extends ChangeNotifier {
  MixarOverlayController() : _inner = AnchorController() {
    _inner.addListener(notifyListeners);
  }

  final AnchorController _inner;

  /// Underlying controller for Mixar shell widgets (`MixarPopover`, etc.).
  AnchorController get anchor => _inner;

  bool get isShowing => _inner.isShowing;

  void show() => _inner.show();

  void hide() => _inner.hide();

  void toggle() => _inner.toggle();

  @override
  void dispose() {
    _inner.removeListener(notifyListeners);
    _inner.dispose();
    super.dispose();
  }
}
