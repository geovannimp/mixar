import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/settings/ui_language.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';

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
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Builder(
          builder: (context) =>
              Text(AppLocalizations.of(context)!.settingsTitle),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('CONFIGURAÇÕES'), findsOneWidget);
  });
}
