import 'package:flutter/widgets.dart';

/// Carries the Settings sidebar query into the active panel so its cards can
/// narrow themselves to the matching entries.
///
/// Empty [query] means "show everything".
class SettingsSearchScope extends InheritedWidget {
  const new({required this.query, required super.child, super.key});

  final String query;

  static String queryOf(BuildContext context) =>
      context
          .dependOnInheritedWidgetOfExactType<SettingsSearchScope>()
          ?.query ??
      '';

  @override
  bool updateShouldNotify(SettingsSearchScope oldWidget) =>
      query != oldWidget.query;
}
