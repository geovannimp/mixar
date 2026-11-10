import 'package:flutter/widgets.dart';

/// A settings card that can decide whether it matches the sidebar search.
///
/// Cards that do not implement this are treated as non-matching; the section
/// keeps them visible when no card advertises the query.
abstract interface class SettingsSearchableCard {
  /// [foldedQuery] is already lowercased and accent-folded by
  /// `foldSettingsSearchText`.
  bool matchesSettingsSearch(BuildContext context, String foldedQuery);
}
