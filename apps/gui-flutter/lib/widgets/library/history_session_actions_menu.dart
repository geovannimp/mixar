import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/providers/shell/mixar_overlay_controller.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/widgets/library/history_detail_pane.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_anchor.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_body.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_group.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_item.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

class HistorySessionActionsMenu extends ConsumerWidget {
  const new({required this.sessionId, required this.session});

  final String sessionId;
  final HistorySessionSummary? session;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return MixarMenuAnchor(
      dialogTitle: 'Session actions',
      menuBuilder: (context, controller) => MixarMenuBody(
        groups: [
          MixarMenuGroup(
            children: [
              MixarMenuItem(
                title: const Text('Rename'),
                enabled: session != null,
                onPress: session == null
                    ? null
                    : () {
                        unawaited(
                          afterHistoryMenu(context, controller, () {
                            return HistoryDetailPane.renameSession(
                              context,
                              ref,
                              session!,
                            );
                          }),
                        );
                      },
              ),
              MixarMenuItem(
                title: const Text('Export'),
                onPress: () {
                  unawaited(
                    afterHistoryMenu(context, controller, () {
                      return HistoryDetailPane.exportSession(
                        context,
                        ref,
                        sessionId,
                        sessionTitle: session?.title,
                      );
                    }),
                  );
                },
              ),
              MixarMenuItem(
                title: const Text('Create collection'),
                onPress: () {
                  unawaited(
                    afterHistoryMenu(context, controller, () {
                      return HistoryDetailPane.createCollectionFromHistory(
                        context,
                        ref,
                        sessionId,
                        session?.title,
                      );
                    }),
                  );
                },
              ),
            ],
          ),
          MixarMenuGroup(
            children: [
              MixarMenuItem(
                title: const Text('Delete'),
                destructive: true,
                onPress: () {
                  unawaited(
                    afterHistoryMenu(context, controller, () {
                      return HistoryDetailPane.deleteSession(
                        context,
                        ref,
                        sessionId,
                      );
                    }),
                  );
                },
              ),
            ],
          ),
        ],
      ),
      childBuilder: (context, controller) => AppButton.icon(
        variant: .ghost,
        semanticsLabel: 'Session actions',
        onPress: controller.toggle,
        child: const Icon(LucideIcons.ellipsisVertical),
      ),
    );
  }
}

Future<void> afterHistoryMenu(
  BuildContext context,
  MixarOverlayController controller,
  Future<void> Function() action,
) async {
  controller.hide();
  if (!context.mounted) {
    return;
  }
  await action();
}
