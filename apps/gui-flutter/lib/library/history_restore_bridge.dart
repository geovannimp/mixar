import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/library/history_providers.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/shell/app_button.dart';
import 'package:gui_flutter/shell/mixar_dialog.dart';

/// Prompt to restore the previous session on launch (inside idle window).
class HistoryRestoreBridge extends ConsumerStatefulWidget {
  const HistoryRestoreBridge({super.key});

  @override
  ConsumerState<HistoryRestoreBridge> createState() =>
      _HistoryRestoreBridgeState();
}

class _HistoryRestoreBridgeState extends ConsumerState<HistoryRestoreBridge> {
  var _prompted = false;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback(
      (_) => unawaited(_maybePrompt()),
    );
  }

  Future<void> _maybePrompt() async {
    if (_prompted || !mounted) {
      return;
    }
    try {
      final library = await ref.read(libraryTransportProvider.future);
      final prompt = await library.historyRestorePrompt();
      if (!mounted || prompt == null) {
        return;
      }
      _prompted = true;
      final restore = await showMixarConfirm<bool>(
        context: context,
        title: 'Restore session?',
        body:
            '“${prompt.title}” was still active. Restore it or start a new session?',
        actions: const [
          MixarDialogAction(
            label: 'Start new',
            value: false,
            variant: MixarButtonVariant.outline,
          ),
          MixarDialogAction(label: 'Restore', value: true),
        ],
      );
      if (!mounted) {
        return;
      }
      if (restore == true) {
        await library.historyRestoreSession(sessionId: prompt.sessionId);
      } else {
        await library.historyDeclineRestore();
      }
      invalidateHistory(ref);
    } catch (e) {
      if (mounted) {
        ref.read(libraryMessageProvider.notifier).setError('$e');
      }
    }
  }

  @override
  Widget build(BuildContext context) => const SizedBox.shrink();
}
