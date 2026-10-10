import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';

void main() {
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
