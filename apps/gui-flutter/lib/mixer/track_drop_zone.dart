import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/mixer/track_drag.dart';
import 'package:super_drag_and_drop/super_drag_and_drop.dart';

/// Deck drop target: in-app [TrackDragPayload] plus OS `fileUri` audio files.
class TrackDropZone extends ConsumerStatefulWidget {
  const TrackDropZone({required this.deckId, required this.child, super.key});

  final int deckId;
  final Widget child;

  @override
  ConsumerState<TrackDropZone> createState() => _TrackDropZoneState();
}

class _TrackDropZoneState extends ConsumerState<TrackDropZone> {
  var _over = false;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final engine = ref.watch(engineTransportProvider).asData?.value;
    final running = ref.watch(engineRunningProvider);
    final enabled = engine != null && running;

    final highlighted = DecoratedBox(
      decoration: BoxDecoration(
        boxShadow: _over && enabled
            ? [
                BoxShadow(
                  color: theme.colors.primary.withValues(alpha: 0.55),
                  spreadRadius: 2,
                ),
              ]
            : null,
      ),
      child: widget.child,
    );

    if (!enabled) {
      return highlighted;
    }

    return DropRegion(
      formats: const [Formats.fileUri],
      hitTestBehavior: HitTestBehavior.opaque,
      onDropOver: (event) {
        if (!event.session.allowedOperations.contains(DropOperation.copy)) {
          return DropOperation.none;
        }
        for (final item in event.session.items) {
          if (parseTrackDragLocalData(item.localData) != null) {
            return DropOperation.copy;
          }
          if (item.canProvide(Formats.fileUri)) {
            return DropOperation.copy;
          }
        }
        return DropOperation.none;
      },
      onDropEnter: (_) => setState(() => _over = true),
      onDropLeave: (_) => setState(() => _over = false),
      onPerformDrop: (event) async {
        setState(() => _over = false);
        await _performDrop(event);
      },
      child: highlighted,
    );
  }

  Future<void> _performDrop(PerformDropEvent event) async {
    for (final item in event.session.items) {
      final local = parseTrackDragLocalData(item.localData);
      if (local != null) {
        await _load(local);
        return;
      }
    }

    final fileItems = [
      for (final item in event.session.items)
        if (item.dataReader != null && item.canProvide(Formats.fileUri)) item,
    ];
    if (fileItems.isEmpty) {
      return;
    }

    var remaining = fileItems.length;
    var loaded = false;
    for (final item in fileItems) {
      item.dataReader!.getValue<Uri>(
        Formats.fileUri,
        (uri) async {
          remaining--;
          if (!loaded && uri != null) {
            final path = pathFromDroppedUri(uri);
            if (isSupportedAudioPath(path)) {
              loaded = true;
              await _load(payloadFromOsPath(path));
            }
          }
          if (remaining == 0 && !loaded && mounted) {
            showFToast(
              context: context,
              title: const Text('No supported audio files in drop'),
            );
          }
        },
        onError: (_) {
          remaining--;
          if (remaining == 0 && !loaded && mounted) {
            showFToast(
              context: context,
              title: const Text('No supported audio files in drop'),
            );
          }
        },
      );
    }
  }

  Future<void> _load(TrackDragPayload payload) async {
    try {
      await loadPayloadToDeck(ref, widget.deckId, payload);
    } catch (e) {
      if (!mounted) {
        return;
      }
      showFToast(context: context, variant: .destructive, title: Text('$e'));
    }
  }
}
