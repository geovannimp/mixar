import 'package:desktop_drop/desktop_drop.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/models/pad_modes.dart';
import 'package:gui_flutter/core/utils/pad_format.dart';
import 'package:gui_flutter/core/utils/track_drag.dart';
import 'package:gui_flutter/providers/library/drawer_providers.dart';
import 'package:gui_flutter/widgets/mixer/pads/hold_pad_button.dart';
import 'package:gui_flutter/widgets/mixer/pads/pad_button.dart';
import 'package:gui_flutter/widgets/mixer/pads/pad_face.dart';
import 'package:gui_flutter/widgets/mixer/pads/pad_grid.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

class SamplerSlot {
  const new({this.label, this.durationMs, this.path});

  final String? label;
  final int? durationMs;
  final String? path;

  bool get filled =>
      (path != null && path!.isNotEmpty) ||
      (label != null && label!.trim().isNotEmpty);
}

class SamplerBank {
  const new({required this.id, required this.name, this.playMode});

  final String id;
  final String name;

  /// Null = inherit default (`oneshot`).
  final String? playMode;
}

class SamplerPads extends ConsumerWidget {
  const new({
    required this.slots,
    required this.holdLike,
    required this.onPress,
    required this.onRelease,
    this.onAssign,
    this.disabled = false,
    super.key,
  });

  final List<SamplerSlot> slots;
  final bool holdLike;
  final void Function(int slot, bool shift) onPress;
  final ValueChanged<int> onRelease;
  final void Function(int slot, TrackDragPayload payload)? onAssign;
  final bool disabled;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    // `desktop_drop` picks its target by geometry, so a file dropped on the
    // library drawer would land on the pad underneath it. While the drawer is
    // up the pads take no OS files.
    final drawerOpen = ref.watch(libraryDrawerOpenProvider);
    return PadGrid(
      children: [
        for (var slot = 0; slot < 8; slot++)
          _slotPad(context.theme, slot, drawerOpen),
      ],
    );
  }

  Widget _slotPad(MixarThemeData theme, int slot, bool drawerOpen) {
    final sample = slot < slots.length ? slots[slot] : const SamplerSlot();
    final filled = sample.filled;
    final label = sample.label?.trim();
    final tooltip = filled
        ? (holdLike
              ? 'Pad ${slot + 1} — hold to play, shift+hold clear'
              : 'Pad ${slot + 1} — press trigger, shift+press clear')
        : 'Sampler pad ${slot + 1} (drop a track to assign)';

    final accent = filled ? hotCueAccent(slot) : null;
    final pad = HoldPadButton(
      disabled: disabled,
      fillChild: true,
      chrome: accent == null ? null : accentPadChrome(theme, accent),
      tooltip: tooltip,
      onBegin: () => onPress(slot, shiftKeyPressed()),
      onEnd: () => onRelease(slot),
      child: PadFace(
        action: 'SAMPLE',
        indicatorColor: accent?.border,
        label: Text(
          filled && label != null && label.isNotEmpty ? label : '${slot + 1}',
          overflow: TextOverflow.ellipsis,
          style: theme.typography.body.sm.copyWith(fontWeight: FontWeight.w700),
        ),
        footer: filled && sample.durationMs != null
            ? Text(
                formatDeckTimeTenth(sample.durationMs),
                style: padActionStyle(theme).copyWith(letterSpacing: 0),
              )
            : null,
      ),
    );

    final assign = onAssign;
    if (assign == null) {
      return pad;
    }
    return DragTarget<TrackDragPayload>(
      hitTestBehavior: HitTestBehavior.opaque,
      onWillAcceptWithDetails: (_) => !disabled,
      onAcceptWithDetails: (details) {
        if (disabled) {
          return;
        }
        assign(slot, details.data);
      },
      builder: (context, _, _) => DropTarget(
        enable: !disabled && !drawerOpen,
        onDragDone: (details) {
          if (disabled) {
            return;
          }
          _assignOsFile(details, slot, assign);
        },
        child: pad,
      ),
    );
  }
}

/// Assign the first supported audio file of an OS drop to the pad.
void _assignOsFile(
  DropDoneDetails details,
  int slot,
  void Function(int slot, TrackDragPayload payload) assign,
) {
  for (final item in details.files) {
    if (item is! DropItemFile) {
      continue;
    }
    final path = item.path;
    if (!isSupportedAudioPath(path)) {
      continue;
    }
    assign(slot, payloadFromOsPath(path));
    return;
  }
}
