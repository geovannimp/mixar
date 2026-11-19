import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/collection_order.dart';

void main() {
  group('reorderEntryIds', () {
    test('moves a row down the full order', () {
      expect(
        reorderEntryIds(
          membership: const ['a', 'b', 'c', 'd'],
          visible: const ['a', 'b', 'c', 'd'],
          oldIndex: 0,
          newIndex: 2,
        ),
        const ['b', 'c', 'a', 'd'],
      );
    });

    test('moves a row up the full order', () {
      expect(
        reorderEntryIds(
          membership: const ['a', 'b', 'c', 'd'],
          visible: const ['a', 'b', 'c', 'd'],
          oldIndex: 3,
          newIndex: 1,
        ),
        const ['a', 'd', 'b', 'c'],
      );
    });

    test('moves a filtered row next to the visible neighbour it landed on', () {
      expect(
        reorderEntryIds(
          membership: const ['a', 'hidden-1', 'b', 'hidden-2', 'c'],
          visible: const ['a', 'b', 'c'],
          oldIndex: 0,
          newIndex: 1,
        ),
        const ['hidden-1', 'b', 'a', 'hidden-2', 'c'],
      );
      expect(
        reorderEntryIds(
          membership: const ['a', 'hidden-1', 'b', 'hidden-2', 'c'],
          visible: const ['a', 'b', 'c'],
          oldIndex: 2,
          newIndex: 1,
        ),
        const ['a', 'hidden-1', 'c', 'b', 'hidden-2'],
      );
    });

    test('a single visible row cannot move and keeps the membership', () {
      const membership = ['a', 'b', 'c'];
      expect(
        reorderEntryIds(
          membership: membership,
          visible: const ['b'],
          oldIndex: 0,
          newIndex: 0,
        ),
        membership,
      );
    });

    test('dragging the last row past the end appends it', () {
      expect(
        reorderEntryIds(
          membership: const ['a', 'b', 'c'],
          visible: const ['a', 'b', 'c'],
          oldIndex: 0,
          newIndex: 2,
        ),
        const ['b', 'c', 'a'],
      );
    });

    test('out-of-range indices leave the membership unchanged', () {
      const membership = ['a', 'b'];
      expect(
        reorderEntryIds(
          membership: membership,
          visible: const ['a', 'b'],
          oldIndex: 2,
          newIndex: 0,
        ),
        membership,
      );
    });
  });
}
