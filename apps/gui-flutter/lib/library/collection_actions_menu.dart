import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/library/analyze_collection_dialog.dart';
import 'package:gui_flutter/library/collection_actions.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/shell/app_button.dart';
import 'package:gui_flutter/shell/mixar_menu.dart';
import 'package:gui_flutter/shell/mixar_overlay_controller.dart';
import 'package:gui_flutter/shell/mixar_popover.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Three-dot menu for a collection (browse in Drive for folders, queue
/// collection-wide analysis). Lives in the track-list topbar — mirroring the
/// history session actions menu — acting on the active collection.
class CollectionActionsMenu extends ConsumerWidget {
  const new({required this.collection, super.key});

  final LibraryCollectionSummary collection;

  Future<void> _analyze(
    BuildContext context,
    WidgetRef ref,
    MixarOverlayController controller,
  ) async {
    controller.hide();
    if (!context.mounted) {
      return;
    }
    final options = await showAnalyzeCollectionDialog(
      context,
      collectionName: collection.name,
      trackCount: collection.trackCount,
    );
    if (options == null || !context.mounted) {
      return;
    }
    // Select the collection first so the analysis pills land on visible rows.
    ref.read(selectedCollectionIdProvider.notifier).set(collection.id);
    ref
        .read(librarySourceTabProvider.notifier)
        .set(LibrarySourceTab.collections);
    await analyzeCollectionAction(
      ref,
      collection.id,
      force: options.force,
      generateStems: options.generateStems,
    );
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final browsePath = collection.kind == 'folder' ? collection.path : null;
    return MixarMenuAnchor(
      menuBuilder: (context, controller) => MixarMenuBody(
        groups: [
          MixarMenuGroup(
            children: [
              if (browsePath != null)
                MixarMenuItem(
                  title: const Text('Browse in Drive'),
                  onPress: () {
                    controller.hide();
                    ref.read(driveCurrentPathProvider.notifier).set(browsePath);
                    ref
                        .read(librarySourceTabProvider.notifier)
                        .set(LibrarySourceTab.drive);
                  },
                ),
              MixarMenuItem(
                title: const Text('Analyze tracks…'),
                onPress: () {
                  unawaited(_analyze(context, ref, controller));
                },
              ),
            ],
          ),
        ],
      ),
      childBuilder: (context, controller) => AppButton.icon(
        variant: .ghost,
        semanticsLabel: 'Collection actions',
        onPress: controller.toggle,
        child: const Icon(LucideIcons.ellipsisVertical),
      ),
    );
  }
}
