import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/main.dart';
import 'package:gui_flutter/shell/desktop.dart';
import 'package:gui_flutter/src/rust/api/meta.dart';
import 'package:gui_flutter/src/rust/frb_generated.dart';
import 'package:integration_test/integration_test.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(() async {
    debugOverrideDesktopWindow = false;
    await RustLib.init();
  });
  testWidgets('shows shared app title from Rust', (WidgetTester tester) async {
    final title = appDisplayName();
    await tester.pumpWidget(ProviderScope(child: Application(appTitle: title)));
    expect(find.text(title), findsWidgets);
  });
}
