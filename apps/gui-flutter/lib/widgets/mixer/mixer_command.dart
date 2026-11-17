import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/mixar_toast.dart';

Future<void> runMixerCommand(
  BuildContext context,
  Future<void> Function() fn,
) async {
  try {
    await fn();
  } catch (error, stackTrace) {
    debugPrintStack(
      label: 'Mixer command failed: $error',
      stackTrace: stackTrace,
    );
    if (!context.mounted) {
      return;
    }
    showMixarToast(
      context: context,
      variant: MixarToastVariant.destructive,
      title: Text('$error'),
    );
  }
}
