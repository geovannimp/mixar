import 'package:flutter/widgets.dart';

/// Marks menu content rendered inside a mobile [showMixarDialog] so rows can
/// match [MixarSelect] bordered option cards without changing desktop panels.
class MixarMenuDialogChrome extends InheritedWidget {
  const new({required super.child, super.key});

  static bool isActive(BuildContext context) {
    return context
            .dependOnInheritedWidgetOfExactType<MixarMenuDialogChrome>() !=
        null;
  }

  @override
  bool updateShouldNotify(covariant MixarMenuDialogChrome oldWidget) => false;
}
