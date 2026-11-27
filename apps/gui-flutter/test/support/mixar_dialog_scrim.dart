import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/widgets/shell/mixar_dialog.dart';

Iterable<ModalBarrier> _mixarScrims(WidgetTester tester) => tester
    .widgetList<ModalBarrier>(find.byType(ModalBarrier))
    .where((barrier) => barrier.color == kMixarDialogScrimColor);

/// Stays in a Mixar dialog by pressing its dimmed body, then asserts it closed.
///
/// The press lands on the scrim above the card: a narrow bottom-sheet layout
/// can put a tall card over a corner, so a corner press would hit the card.
/// The scrim must be gone afterwards, so a press that only grazed the card
/// fails here instead of passing as a dismissal.
Future<void> dismissMixarDialogScrim(WidgetTester tester) async {
  final scrims = _mixarScrims(tester);
  if (scrims.isEmpty) {
    fail(
      'No Mixar dialog scrim on screen: open the dialog before dismissing it.',
    );
  }
  final barrierRect = tester.getRect(find.byWidget(scrims.first));
  final point = Offset(barrierRect.center.dx, barrierRect.top + 8);
  if (point.dy >= tester.getRect(find.byType(MixarDialog).first).top) {
    fail('The dialog card leaves no scrim above it to press.');
  }
  await tester.tapAt(point);
  await tester.pumpAndSettle();
  if (_mixarScrims(tester).isNotEmpty) {
    fail('The dialog is still open after pressing the scrim.');
  }
}
