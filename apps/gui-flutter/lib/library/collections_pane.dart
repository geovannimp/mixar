import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/src/rust/api/library.dart';

/// Flat collections list ([FItemGroup](https://forui.dev/docs/widgets/data/item-group)).
class CollectionsPane extends ConsumerWidget {
  const CollectionsPane({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = context.theme;
    final colors = theme.colors;
    final collections = ref.watch(collectionsProvider);
    final selectedId = ref.watch(activeCollectionIdProvider);
    // Forui items default to colors.background (near-black). Use secondary, and a
    // slightly lighter fill for hover/selected (muted == secondary in neutral dark).
    final highlight = Color.alphaBlend(
      colors.foreground.withValues(alpha: 0.1),
      colors.secondary,
    );
    final itemStyle = FItemStyleDelta.delta(
      backgroundColor: .delta([
        .base(colors.background.withValues(alpha: 0.00)),
      ]),
      padding: .value(EdgeInsets.zero),
      contentDecoration: .delta([
        .base(.shapeDelta(color: colors.secondary)),
        .match({.hovered}, .shapeDelta(color: highlight)),
        .match({.pressed}, .shapeDelta(color: highlight)),
        .match({.selected}, .shapeDelta(color: highlight)),
      ]),
    );

    void onItemPress(LibraryCollectionSummary c) {
      ref.read(selectedCollectionIdProvider.notifier).state = c.id;
    }

    return Padding(
      padding: const EdgeInsets.all(12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text('Collections', style: theme.typography.display.sm),
          const SizedBox(height: 12),
          Expanded(
            child: collections.when(
              loading: () => const Center(child: FCircularProgress()),
              error: (e, _) => Text(
                'Library error: $e',
                style: theme.typography.body.sm.copyWith(
                  color: colors.destructive,
                ),
              ),
              data: (items) {
                if (items.isEmpty) {
                  return Text(
                    'No collections yet',
                    style: theme.typography.body.sm.copyWith(
                      color: colors.mutedForeground,
                    ),
                  );
                }
                return FItemGroup(
                  children: [
                    for (final c in items)
                      FItem(
                        title: Text(c.name),
                        subtitle: Text('${c.trackCount} tracks'),
                        selected: c.id == selectedId,
                        style: itemStyle,
                        onPress: () => onItemPress(c),
                      ),
                  ],
                );
              },
            ),
          ),
        ],
      ),
    );
  }
}
