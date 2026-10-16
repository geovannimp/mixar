import 'package:flutter/widgets.dart';

/// Whether a drag is currently over this deck's [TrackDropZone].
///
/// The drop target stays deck-wide; the track-info card reads this to paint
/// the hover chrome without lighting up the whole deck.
class DeckDropHover extends InheritedWidget {
  const new({required this.active, required super.child, super.key});

  final bool active;

  static bool of(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<DeckDropHover>()?.active ??
      false;

  @override
  bool updateShouldNotify(DeckDropHover oldWidget) =>
      active != oldWidget.active;
}
