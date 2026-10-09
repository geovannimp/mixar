import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/settings/mixxx_import_panel.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  testWidgets('mixxx import panel shows the import action', (tester) async {
    final theme = MixarThemeData.dark();
    await tester.pumpWidget(
      ProviderScope(
        child: MaterialApp(
          theme: materialUiThemeFromMixar(theme),
          builder: mixarMaterialAppBuilder(theme),
          home: const Scaffold(body: MixxxImportPanel()),
        ),
      ),
    );
    expect(find.text('Import from Mixxx'), findsOneWidget);
    expect(find.text('Import from Mixxx library…'), findsOneWidget);
  }, semanticsEnabled: false);
}
