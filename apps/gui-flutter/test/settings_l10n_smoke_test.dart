import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/models/settings_section.dart';
import 'package:gui_flutter/core/utils/ui_language.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_sidebar.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
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

  testWidgets('settingsTitle localizes under pt_BR and differs from en', (
    tester,
  ) async {
    await tester.pumpWidget(
      mixarTestMaterialApp(
        theme: MixarThemeData.dark(),
        locale: const Locale('pt', 'BR'),
        home: Builder(
          builder: (context) =>
              Text(AppLocalizations.of(context)!.settingsTitle),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('CONFIGURAÇÕES'), findsOneWidget);
    expect(find.text('SETTINGS'), findsNothing);

    await tester.pumpWidget(
      mixarTestMaterialApp(
        theme: MixarThemeData.dark(),
        locale: const Locale('en'),
        home: Builder(
          builder: (context) =>
              Text(AppLocalizations.of(context)!.settingsTitle),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('SETTINGS'), findsOneWidget);
    expect(find.text('CONFIGURAÇÕES'), findsNothing);
  });

  testWidgets('SettingsSidebar chrome follows pt_BR locale', (tester) async {
    final theme = MixarThemeData.dark();
    await tester.pumpWidget(
      mixarTestMaterialApp(
        theme: theme,
        locale: const Locale('pt', 'BR'),
        home: Scaffold(
          body: SettingsSidebar(active: SettingsSection.ui, onSelect: (_) {}),
        ),
      ),
    );
    await tester.pumpAndSettle();
    final l10n = lookupAppLocalizations(const Locale('pt', 'BR'));
    for (final section in kSettingsSections) {
      expect(find.text(section.label(l10n)), findsOneWidget);
    }
    expect(find.text('UI'), findsNothing);
  });
}
