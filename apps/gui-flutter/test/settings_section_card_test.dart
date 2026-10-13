import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/settings/settings_field.dart';
import 'package:gui_flutter/settings/settings_widgets.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  testWidgets('renders title, subtitle, and child', (tester) async {
    await tester.pumpWidget(
      mixarTestMaterialApp(
        theme: MixarThemeData.dark(),
        home: const Scaffold(
          body: SettingsSectionCard(
            title: 'Engine',
            subtitle: 'Backend and buffer.',
            child: Text('field-body'),
          ),
        ),
      ),
    );
    expect(find.text('Engine'), findsOneWidget);
    expect(find.text('Backend and buffer.'), findsOneWidget);
    expect(find.text('field-body'), findsOneWidget);
    expect(find.byType(SettingsSectionHeader), findsOneWidget);
    expect(find.byType(Row), findsNothing);
  }, semanticsEnabled: false);

  testWidgets('places trailing beside the header', (tester) async {
    await tester.pumpWidget(
      mixarTestMaterialApp(
        theme: MixarThemeData.dark(),
        home: const Scaffold(
          body: SettingsSectionCard(
            title: 'Mappings',
            subtitle: 'Stored maps.',
            trailing: Text('Update All'),
            child: Text('list'),
          ),
        ),
      ),
    );
    expect(find.text('Update All'), findsOneWidget);
    expect(find.text('list'), findsOneWidget);
    expect(find.byType(Row), findsOneWidget);
  }, semanticsEnabled: false);

  testWidgets('omits body spacing when child is null', (tester) async {
    await tester.pumpWidget(
      mixarTestMaterialApp(
        theme: MixarThemeData.dark(),
        home: const Scaffold(
          body: SettingsSectionCard(
            title: 'Preview bus',
            subtitle: 'Headphones / cue.',
            trailing: Text('toggle'),
          ),
        ),
      ),
    );
    expect(find.text('Preview bus'), findsOneWidget);
    expect(find.text('toggle'), findsOneWidget);
    expect(find.text('field-body'), findsNothing);
  }, semanticsEnabled: false);
}
