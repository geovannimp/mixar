import 'package:flutter/material.dart' as material;
import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/shell/app_button.dart';
import 'package:gui_flutter/shell/legacy_material_scope.dart';
import 'package:wolt_modal_sheet/wolt_modal_sheet.dart';

/// Wolt medium breakpoint — dialog above, bottom sheet below.
const mixarDialogBreakpoint = 768.0;

class MixarDialogAction<T> {
  const MixarDialogAction({
    required this.label,
    required this.value,
    this.variant = MixarButtonVariant.primary,
  });

  final String label;
  final T value;
  final MixarButtonVariant variant;
}

WoltModalType mixarModalTypeBuilder(BuildContext context) {
  final width = MediaQuery.sizeOf(context).width;
  return width < mixarDialogBreakpoint
      ? WoltModalType.bottomSheet()
      : WoltModalType.dialog();
}

Future<T?> _showMixarModal<T>({
  required BuildContext context,
  required List<SliverWoltModalSheetPage> Function(BuildContext)
  pageListBuilder,
}) {
  // wolt_modal_sheet reads package:flutter/material MaterialLocalizations;
  // belt-and-suspenders with LegacyMaterialScope for the modal subtree.
  return WoltModalSheet.show<T>(
    context: context,
    modalTypeBuilder: mixarModalTypeBuilder,
    barrierDismissible: true,
    modalDecorator: (child) {
      return material.Localizations.override(
        context: context,
        delegates: const [
          LegacyMaterialScope.flutterMaterialLocalizationsDelegate,
        ],
        child: child,
      );
    },
    pageListBuilder: pageListBuilder,
  );
}

/// Confirmation with Wolt page title + sticky action bar.
Future<T?> showMixarConfirm<T>({
  required BuildContext context,
  required String title,
  String? body,
  required List<MixarDialogAction<T>> actions,
}) {
  assert(actions.isNotEmpty, 'showMixarConfirm requires at least one action');
  return _showMixarModal<T>(
    context: context,
    pageListBuilder: (modalContext) {
      final theme = modalContext.theme;
      return [
        WoltModalSheetPage(
          hasSabGradient: false,
          isTopBarLayerAlwaysVisible: true,
          topBarTitle: Text(
            title,
            style: theme.typography.body.md.copyWith(
              fontWeight: FontWeight.w700,
            ),
          ),
          child: Padding(
            padding: const EdgeInsets.fromLTRB(16, 16, 16, 16),
            child: body == null
                ? const SizedBox.shrink()
                : Text(body, style: theme.typography.body.sm),
          ),
          stickyActionBar: Padding(
            padding: const EdgeInsets.all(16),
            child: Row(
              mainAxisAlignment: MainAxisAlignment.end,
              spacing: 8,
              children: [
                for (final action in actions)
                  AppButton(
                    mainAxisSize: MainAxisSize.min,
                    variant: action.variant,
                    onPress: () => Navigator.of(modalContext).pop(action.value),
                    child: Text(action.label),
                  ),
              ],
            ),
          ),
        ),
      ];
    },
  );
}

/// Content-owned dialog (forms / pickers). Call site supplies title + actions.
Future<T?> showMixarDialog<T>({
  required BuildContext context,
  required WidgetBuilder builder,
}) {
  return _showMixarModal<T>(
    context: context,
    pageListBuilder: (modalContext) => [
      WoltModalSheetPage(
        hasTopBarLayer: false,
        hasSabGradient: false,
        child: builder(modalContext),
      ),
    ],
  );
}
