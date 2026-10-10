import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:material_ui/material_ui.dart';

/// Delegates for Mixar: app ARBs + material_ui kit localizations.
///
/// Prefer material_ui's [GlobalMaterialLocalizations] (not Flutter's) so the
/// kit's `MaterialLocalizations` type resolves for non-English locales.
List<LocalizationsDelegate<dynamic>> get mixarLocalizationsDelegates => [
  AppLocalizations.delegate,
  ...GlobalMaterialLocalizations.delegates,
];
