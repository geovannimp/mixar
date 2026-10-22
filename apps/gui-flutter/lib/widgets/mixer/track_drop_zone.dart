import 'dart:async';

import 'package:desktop_drop/desktop_drop.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/utils/track_drag.dart';
import 'package:gui_flutter/providers/library/drawer_providers.dart';
import 'package:gui_flutter/providers/mixer/deck_load.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/widgets/mixer/deck_drop_hover.dart';
import 'package:gui_flutter/widgets/shell/mixar_toast.dart';

/// Deck drop target: in-app [TrackDragPayload] plus OS audio files.
///
/// Two mechanisms, because they are two different things: a `DragTarget`
/// takes the track a row in the library hands over inside the app, and
/// [DropTarget] takes files dragged in from the desktop.
class TrackDropZone extends ConsumerStatefulWidget {
  const new({required this.deckId, required this.child, super.key});

  final int deckId;
  final Widget child;

  @override
  ConsumerState<TrackDropZone> createState() => _TrackDropZoneState();
}

class _TrackDropZoneState extends ConsumerState<TrackDropZone> {
  var _over = false;

  void _setOver(bool value) {
    if (_over == value) {
      return;
    }
    setState(() => _over = value);
  }

  @override
  Widget build(BuildContext context) {
    final engine = ref.watch(engineTransportProvider).asData?.value;
    final running = ref.watch(engineRunningProvider);
    final enabled = engine != null && running;
    // The drawer floats over the mixer, and `desktop_drop` picks its target by
    // geometry, so a file dropped on the drawer would land on the deck
    // underneath it. While the drawer is up the deck takes no OS files.
    final drawerOpen = ref.watch(libraryDrawerOpenProvider);

    final content = DeckDropHover(
      active: _over && enabled,
      child: widget.child,
    );

    if (!enabled) {
      return content;
    }

    return DragTarget<TrackDragPayload>(
      hitTestBehavior: HitTestBehavior.opaque,
      onWillAcceptWithDetails: (_) => true,
      onMove: (_) => _setOver(true),
      onLeave: (_) => _setOver(false),
      onAcceptWithDetails: (details) {
        _setOver(false);
        unawaited(_load(details.data));
      },
      builder: (context, _, _) => DropTarget(
        enable: !drawerOpen,
        onDragEntered: (_) => _setOver(true),
        onDragExited: (_) => _setOver(false),
        onDragDone: _onOsDrop,
        child: content,
      ),
    );
  }

  void _onOsDrop(DropDoneDetails details) {
    _setOver(false);
    final paths = <String>[
      for (final item in details.files)
        if (item is DropItemFile && isSupportedAudioPath(item.path)) item.path,
    ];
    if (paths.isEmpty) {
      showMixarToast(
        context: context,
        title: const Text('No supported audio files in drop'),
      );
      return;
    }
    unawaited(_load(payloadFromOsPath(paths.first)));
  }

  Future<void> _load(TrackDragPayload payload) async {
    try {
      await loadPayloadToDeck(ref, widget.deckId, payload);
    } catch (e) {
      if (!mounted) {
        return;
      }
      showMixarToast(
        context: context,
        variant: MixarToastVariant.destructive,
        title: Text('$e'),
      );
    }
  }
}
