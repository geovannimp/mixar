import 'package:gui_flutter/src/rust/api/settings.dart';

/// Whether [field] is in the saved, ordered selection.
bool libraryRowFieldShown(
  List<LibraryRowFieldSetting> fields,
  LibraryRowFieldSetting field,
) => fields.contains(field);

/// The saved selection with [field] added or removed.
///
/// Enabling appends, so a newly shown field takes the lowest overflow
/// priority — the chip that disappears first in a narrow pane is the one just
/// turned on, not one the user already had.
List<LibraryRowFieldSetting> toggleLibraryRowField(
  List<LibraryRowFieldSetting> fields,
  LibraryRowFieldSetting field, {
  required bool shown,
}) {
  final next = [
    for (final existing in fields)
      if (existing != field) existing,
  ];
  if (shown) {
    next.add(field);
  }
  return next;
}
