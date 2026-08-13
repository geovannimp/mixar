import 'package:file_picker/file_picker.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/src/rust/api/library.dart';

/// Flat collections list ([FItemGroup](https://forui.dev/docs/widgets/data/item-group)).
class CollectionsPane extends ConsumerStatefulWidget {
  const CollectionsPane({super.key});

  @override
  ConsumerState<CollectionsPane> createState() => _CollectionsPaneState();
}

class _CollectionsPaneState extends ConsumerState<CollectionsPane> {
  var _adding = false;

  Future<void> _addFolder() async {
    final path = await FilePicker.platform.getDirectoryPath();
    if (path == null || !mounted) {
      return;
    }
    setState(() => _adding = true);
    ref.read(libraryMessageProvider.notifier).clear();
    try {
      final transport = await ref.read(libraryTransportProvider.future);
      final result = await transport.addFolderCollection(folderPath: path);
      ref.invalidate(collectionsProvider);
      ref.invalidate(collectionTracksProvider);
      ref.read(selectedCollectionIdProvider.notifier).set(result.collection.id);
    } catch (e) {
      ref.read(libraryMessageProvider.notifier).setError('$e');
    } finally {
      if (mounted) {
        setState(() => _adding = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final colors = theme.colors;
    final collections = ref.watch(collectionsProvider);
    final selectedId = ref.watch(activeCollectionIdProvider);
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
      ref.read(selectedCollectionIdProvider.notifier).set(c.id);
    }

    return Padding(
      padding: const EdgeInsets.all(12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Align(
            alignment: Alignment.centerRight,
            child: FButton(
              size: .sm,
              variant: .primary,
              onPress: _adding ? null : _addFolder,
              child: _adding
                  ? const FCircularProgress(size: .sm)
                  : const Text('Add folder'),
            ),
          ),
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
