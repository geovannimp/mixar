import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/widgets/shell/mixar_dialog.dart';

/// Stays in a Mixar dialog by pressing its dimmed body, then settles.
///
/// The point sits at the barrier's own top-left corner, so it holds for the
/// narrow bottom-sheet layout as well as the centered card. Callers assert
/// what staying meant (the draft kept, the page still open) afterwards.
Future<void> dismissMixarDialogScrim(WidgetTester tester) async {
  final barrier = tester
      .widgetList<ModalBarrier>(find.byType(ModalBarrier))
      .firstWhere((b) => b.color == kMixarDialogScrimColor);
  final barrierRect = tester.getRect(find.byWidget(barrier));
  await tester.tapAt(barrierRect.topLeft + const Offset(8, 8));
  await tester.pumpAndSettle();
}
