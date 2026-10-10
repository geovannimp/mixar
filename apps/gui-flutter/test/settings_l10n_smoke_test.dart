import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/settings/settings_section.dart';
import 'package:gui_flutter/settings/settings_sidebar.dart';
import 'package:gui_flutter/settings/ui_language.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  test('localeFromUiLanguage maps preference to MaterialApp.locale', () {
    expect(localeFromUiLanguage(UiLanguageSetting.system), isNull);
    expect(localeFromUiLanguage(UiLanguageSetting.en), const Locale('en'));
    expect(
      localeFromUiLanguage(UiLanguageSetting.ptBr),
      const Locale('pt', 'BR'),
    );
  });

  testWidgets('settingsTitle localizes under pt_BR', (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        locale: const Locale('pt', 'BR'),
        localizationsDelegates: mixarTestLocalizationsDelegates,
        supportedLocales: mixarTestSupportedLocales,
        home: Builder(
          builder: (context) =>
              Text(AppLocalizations.of(context)!.settingsTitle),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('CONFIGURAÇÕES'), findsOneWidget);
  });

  testWidgets('SettingsSidebar chrome follows pt_BR locale', (tester) async {
    final theme = MixarThemeData.dark();
    await tester.pumpWidget(
      MaterialApp(
        locale: const Locale('pt', 'BR'),
        localizationsDelegates: mixarTestLocalizationsDelegates,
        supportedLocales: mixarTestSupportedLocales,
        theme: materialUiThemeFromMixar(theme),
        builder: mixarMaterialAppBuilder(theme),
        home: Scaffold(
          body: SettingsSidebar(active: SettingsSection.ui, onSelect: (_) {}),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('Interface'), findsOneWidget);
    expect(find.text('Biblioteca'), findsOneWidget);
    expect(find.text('UI'), findsNothing);
  });
}
