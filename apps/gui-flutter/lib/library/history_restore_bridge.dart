import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/library/history_providers.dart';
import 'package:gui_flutter/library/providers.dart';

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
      final restore = await showFDialog<bool>(
        context: context,
        builder: (context, _, animation) {
          return FDialog(
            animation: animation,
            builder: (context, _) {
              final theme = context.theme;
              return Padding(
                padding: const EdgeInsets.all(16),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    Text(
                      'Restore session?',
                      style: theme.typography.body.md.copyWith(
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                    const SizedBox(height: 8),
                    Text(
                      '“${prompt.title}” was still active. Restore it or start a new session?',
                    ),
                    const SizedBox(height: 16),
                    Row(
                      spacing: 8,
                      children: [
                        FButton(
                          variant: .outline,
                          onPress: () => Navigator.of(context).pop(false),
                          child: const Text('Start new'),
                        ),
                        FButton(
                          onPress: () => Navigator.of(context).pop(true),
                          child: const Text('Restore'),
                        ),
                      ],
                    ),
                  ],
                ),
              );
            },
          );
        },
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
