import 'package:flutter/widgets.dart';
import 'package:gui_flutter/src/rust/api/library.dart';

/// Canonical colour-label names, in the device table's order (pink first).
///
/// The set is closed on the Rust side too (`library_core::ColorLabel`), so an
/// arbitrary hex value has no representation to round-trip.
final colorLabelNames = LibraryColorLabel.values;

/// Display label for a canonical name (`"pink"` → `"Pink"`).
String colorLabelDisplayName(LibraryColorLabel? label) {
  if (label == null) {
    return 'None';
  }
  final name = label.name;
  return name[0].toUpperCase() + name.substring(1);
}

/// Swatch colour for a canonical name; `null` for an unset/unknown label.
Color? colorLabelColor(LibraryColorLabel? label) => switch (label) {
  null => null,
  LibraryColorLabel.pink => const Color(0xFFFF0080),
  LibraryColorLabel.red => const Color(0xFFFF0000),
  LibraryColorLabel.orange => const Color(0xFFFF8000),
  LibraryColorLabel.yellow => const Color(0xFFFFFF00),
  LibraryColorLabel.green => const Color(0xFF00FF00),
  LibraryColorLabel.aqua => const Color(0xFF00FFFF),
  LibraryColorLabel.blue => const Color(0xFF0000FF),
  LibraryColorLabel.purple => const Color(0xFF8000FF),
};
