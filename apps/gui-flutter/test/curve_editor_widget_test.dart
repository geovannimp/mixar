import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/models/curve_editor.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/src/rust/api/curve_editor.dart';
import 'package:gui_flutter/src/rust/api/event_mirrors.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/curves/curve_group_editor.dart';
import 'package:gui_flutter/widgets/curves/curve_import_export_dialog.dart';
import 'package:gui_flutter/widgets/settings/settings_curves_panel.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/mixar_input.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

class _Harness extends StatefulWidget {
  const new({this.importer, this.exporter});

  final VirtualDjCurveImporter? importer;
  final VirtualDjCurveExporter? exporter;

  @override
  State<_Harness> createState() => _HarnessState();
}

class _HarnessState extends State<_Harness> {
  AppSettings _settings = defaultAppSettings();

  @override
  Widget build(BuildContext context) {
    return ProviderScope(
      child: mixarTestMaterialApp(
        theme: MixarThemeData.dark(),
        home: Scaffold(
          body: SingleChildScrollView(
            child: SettingsCurvesPanel(
              draft: _settings,
              importer: widget.importer,
              exporter: widget.exporter,
              onChanged: (next) => setState(() => _settings = next),
            ),
          ),
        ),
      ),
    );
  }
}

void main() {
  testWidgets('creates, renames, and deletes a custom curve', (tester) async {
    await tester.pumpWidget(const _Harness());
    await tester.pumpAndSettle();

    expect(find.text('No custom curves yet.'), findsOneWidget);

    await tester.tap(find.text('New curve'));
    await tester.pumpAndSettle();
    expect(find.text('New curve'), findsWidgets);
    await tester.enterText(find.byType(MixarInput).first, 'My Curve');
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(AppButton, 'Create'));
    await tester.pumpAndSettle();

    expect(find.text('My Curve'), findsOneWidget);
    expect(find.text('my_curve'), findsOneWidget);
    expect(find.byType(CurveGroupEditor), findsOneWidget);

    await tester.tap(find.byIcon(LucideIcons.pencil).first);
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(MixarInput).first, 'Renamed Curve');
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(AppButton, 'Save'));
    await tester.pumpAndSettle();
    expect(find.text('Renamed Curve'), findsOneWidget);
    expect(find.text('my_curve'), findsOneWidget);

    await tester.tap(find.byIcon(LucideIcons.trash2).first);
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(AppButton, 'Delete'));
    await tester.pumpAndSettle();
    expect(find.text('No custom curves yet.'), findsOneWidget);
  }, semanticsEnabled: false);

  testWidgets('adds and removes a point from the selected curve', (
    tester,
  ) async {
    final changes = <CurveGroupSpec>[];
    final group = newCurveGroup(id: 'mine', displayName: 'Mine', channels: 2);
    await tester.pumpWidget(
      ProviderScope(
        child: mixarTestMaterialApp(
          theme: MixarThemeData.dark(),
          home: Scaffold(
            body: SingleChildScrollView(
              child: CurveGroupEditor(group: group, onChanged: changes.add),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Deck A'), findsOneWidget);
    expect(find.text('Deck B'), findsOneWidget);

    final remove = tester.widget<AppButton>(
      find.widgetWithText(AppButton, 'Remove point'),
    );
    expect(remove.onPress, isNull, reason: 'an endpoint cannot be removed');

    await tester.tap(find.widgetWithText(AppButton, 'Add point'));
    await tester.pumpAndSettle();
    expect(changes.last.curves.first.points.length, 3);
    expect(changes.last.curves.first.segments.length, 2);

    await tester.tap(find.text('Deck B'));
    await tester.pumpAndSettle();
    expect(find.text('Deck B'), findsOneWidget);
  }, semanticsEnabled: false);

  testWidgets('imports and exports through the injected bridge hooks', (
    tester,
  ) async {
    final imported = newCurveGroup(
      id: '',
      displayName: 'VirtualDJ custom',
      channels: 2,
    );
    Future<VirtualDjCurveImport> importer({
      required String crossfaderCurve,
      required String crossfaderCustom,
    }) async {
      expect(crossfaderCurve, 'Custom');
      expect(crossfaderCustom, '0=[1,0]/1=[0,1]');
      return VirtualDjCurveImport(
        group: imported,
        suggestedName: 'VirtualDJ custom',
        crossfaderCurve: 'Custom',
        hamster: false,
        disabled: false,
      );
    }

    Future<VirtualDjCurveExport> exporter({
      required CurveGroupSpec group,
    }) async {
      return const VirtualDjCurveExport(
        crossfaderCurve: 'Custom',
        crossfaderCustom: '0=[1,0]/1=[0,1]',
      );
    }

    await tester.pumpWidget(_Harness(importer: importer, exporter: exporter));
    await tester.pumpAndSettle();

    await tester.tap(find.widgetWithText(AppButton, 'Import VirtualDJ'));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(MixarInput).first, 'Custom');
    await tester.enterText(find.byType(MixarInput).at(1), '0=[1,0]/1=[0,1]');
    await tester.tap(find.widgetWithText(AppButton, 'Import VirtualDJ').last);
    await tester.pumpAndSettle();
    expect(find.text('VirtualDJ custom'), findsOneWidget);

    await tester.pump(const Duration(seconds: 6));
    await tester.pumpAndSettle();
    await tester.ensureVisible(
      find.widgetWithText(AppButton, 'Export to VirtualDJ'),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(AppButton, 'Export to VirtualDJ'));
    await tester.pumpAndSettle();
    expect(find.text('0=[1,0]/1=[0,1]'), findsOneWidget);
  }, semanticsEnabled: false);
}
