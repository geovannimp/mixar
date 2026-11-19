/// Reorder helpers for sortable playlists.
///
/// The stored order is the playlist's whole membership; the list the user
/// drags may be filtered. These functions translate a move in the visible rows
/// into the new full membership, so hidden rows keep their places.
library;

/// Apply a visible-list move to [membership], returning the new full order.
///
/// [membership] and [visible] are collection-entry ids in display order;
/// [visible] is a subsequence of [membership]. [oldIndex] and [newIndex] index
/// [visible] with the adjusted semantics `ReorderableList.onReorderItem`
/// reports: [newIndex] is the slot the moved row occupies after it is taken
/// out of [oldIndex].
///
/// The moved entry is placed next to the visible neighbour it was dropped
/// against, so a filter showing every other row cannot reshuffle the hidden
/// ones. An out-of-range move returns [membership] unchanged.
List<String> reorderEntryIds({
  required List<String> membership,
  required List<String> visible,
  required int oldIndex,
  required int newIndex,
}) {
  if (oldIndex < 0 ||
      oldIndex >= visible.length ||
      newIndex < 0 ||
      newIndex >= visible.length ||
      oldIndex == newIndex) {
    return membership;
  }

  final moved = visible[oldIndex];
  final reordered = [...visible]
    ..removeAt(oldIndex)
    ..insert(newIndex, moved);

  // The row now sits after `before` and before `after`. A downward move hugs
  // `before` and an upward move hugs `after`, so the row lands next to the
  // visible neighbour it was dropped against instead of jumping past rows the
  // filter hides.
  final before = newIndex > 0 ? reordered[newIndex - 1] : null;
  final after = newIndex + 1 < reordered.length
      ? reordered[newIndex + 1]
      : null;

  final order = [
    for (final id in membership)
      if (id != moved) id,
  ];
  int? anchor(String? id, {required bool below}) {
    if (id == null) {
      return null;
    }
    final index = order.indexOf(id);
    return index < 0 ? null : (below ? index + 1 : index);
  }

  final movedDown = newIndex > oldIndex;
  final primary = anchor(movedDown ? before : after, below: movedDown);
  final secondary = anchor(movedDown ? after : before, below: !movedDown);
  order.insert(primary ?? secondary ?? order.length, moved);
  return order;
}
