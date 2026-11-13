import 'package:flutter_test/flutter_test.dart';

/// Two taps at [first] (or [second]) inside the double-tap window.
///
/// The 50 ms gap clears `kDoubleTapMinTime` while staying under
/// `kDoubleTapTimeout`, so the recognizer sees a double tap.
Future<void> doubleTapAt(
  WidgetTester tester,
  Offset first, [
  Offset? second,
]) async {
  await tester.tapAt(first);
  await tester.pump(const Duration(milliseconds: 50));
  await tester.tapAt(second ?? first);
  await tester.pumpAndSettle();
}
