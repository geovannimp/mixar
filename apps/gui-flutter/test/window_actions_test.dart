import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/widgets/shell/window_actions.dart';

void main() {
  test(
    'closeApp leaves the host alone without a native window',
    // Widget tests (and any headless host) have no window: the helper must
    // return instead of ending the test process.
    closeApp,
  );
}
