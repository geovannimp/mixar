import 'package:flutter/widgets.dart';

/// Whether the library surface is rendering its narrow two-page layout.
///
/// The surface measures its own width (`>= [kLibraryWideMinWidth]` is the
/// split), not the window's, so the same rule covers the docked panel and the
/// drawer at any edge. Rows inside the surface read this to decide whether a
/// press navigates to the content page.
class LibraryLayoutScope extends InheritedWidget {
  const new({required this.narrow, required super.child, super.key});

  final bool narrow;

  static LibraryLayoutScope of(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<LibraryLayoutScope>()!;

  static LibraryLayoutScope? maybeOf(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<LibraryLayoutScope>();

  @override
  bool updateShouldNotify(LibraryLayoutScope oldWidget) =>
      oldWidget.narrow != narrow;
}
