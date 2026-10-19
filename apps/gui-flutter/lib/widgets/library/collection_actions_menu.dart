import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/utils/collection_actions.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/providers/shell/mixar_overlay_controller.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/widgets/library/analyze_collection_dialog.dart';
import 'package:gui_flutter/widgets/library/export_cdj_usb_dialog.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_anchor.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_body.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_group.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_item.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Three-dot menu for a collection (browse in Drive for folders, queue
/// collection-wide analysis, export to a CDJ USB drive). Lives in the
/// track-list topbar — mirroring the history session actions menu — acting on
/// the active collection.
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

  Future<void> _exportCdjUsb(
    BuildContext context,
    MixarOverlayController controller,
  ) async {
    controller.hide();
    if (!context.mounted) {
      return;
    }
    await showExportCdjUsbDialog(context, collection: collection);
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final browsePath = collection.kind == 'folder' ? collection.path : null;
    return MixarMenuAnchor(
      dialogTitle: 'Collection actions',
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
              MixarMenuItem(
                title: const Text('Export to CDJ USB…'),
                onPress: () {
                  unawaited(_exportCdjUsb(context, controller));
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
