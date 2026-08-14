import 'dart:async';

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
      formats: const [Formats.plainText, Formats.fileUri],
      hitTestBehavior: HitTestBehavior.opaque,
      onDropOver: (event) {
        if (!_canAcceptNative(event)) {
          return DropOperation.none;
        }
        final op = _dropOperation(event.session.allowedOperations);
        return op == DropOperation.none ? DropOperation.copy : op;
      },
      onDropEnter: (_) => setState(() => _over = true),
      onDropLeave: (_) => setState(() => _over = false),
      onPerformDrop: (event) async {
        setState(() => _over = false);
        _performDrop(event);
      },
      child: highlighted,
    );
  }

  bool _canAcceptNative(DropOverEvent event) {
    for (final item in event.session.items) {
      if (parseTrackDragLocalData(item.localData) != null) {
        return true;
      }
      if (item.canProvide(Formats.plainText) ||
          item.canProvide(Formats.fileUri)) {
        return true;
      }
    }
    return false;
  }

  DropOperation _dropOperation(Set<DropOperation> allowed) {
    switch (preferredTrackDropOperation(allowed.map((op) => op.name))) {
      case 'copy':
        return DropOperation.copy;
      case 'move':
        return DropOperation.move;
      case 'link':
        return DropOperation.link;
      default:
        return DropOperation.copy;
    }
  }

  void _performDrop(PerformDropEvent event) {
    var loaded = false;
    void tryLoad(TrackDragPayload? payload) {
      if (loaded || payload == null) {
        return;
      }
      loaded = true;
      unawaited(_load(payload));
    }

    for (final item in event.session.items) {
      tryLoad(parseTrackDragLocalData(item.localData));
      if (loaded) {
        return;
      }
    }

    final fileItems = <DropItem>[];
    for (final item in event.session.items) {
      final reader = item.dataReader;
      if (reader == null) {
        continue;
      }
      if (item.canProvide(Formats.plainText)) {
        reader.getValue<String>(
          Formats.plainText,
          (text) => tryLoad(parseTrackDragPlainText(text)),
        );
      }
      if (item.canProvide(Formats.fileUri)) {
        fileItems.add(item);
      }
    }

    if (fileItems.isEmpty) {
      return;
    }

    var remaining = fileItems.length;
    for (final item in fileItems) {
      item.dataReader!.getValue<Uri>(
        Formats.fileUri,
        (uri) {
          remaining--;
          if (!loaded && uri != null) {
            final path = pathFromDroppedUri(uri);
            if (isSupportedAudioPath(path)) {
              tryLoad(payloadFromOsPath(path));
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
