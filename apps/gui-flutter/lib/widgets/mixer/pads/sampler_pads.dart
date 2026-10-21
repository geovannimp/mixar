import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/models/pad_modes.dart';
import 'package:gui_flutter/core/utils/pad_format.dart';
import 'package:gui_flutter/core/utils/track_drag.dart';
import 'package:gui_flutter/widgets/mixer/pads/hold_pad_button.dart';
import 'package:gui_flutter/widgets/mixer/pads/pad_button.dart';
import 'package:gui_flutter/widgets/mixer/pads/pad_face.dart';
import 'package:gui_flutter/widgets/mixer/pads/pad_grid.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:super_drag_and_drop/super_drag_and_drop.dart';

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

class SamplerPads extends StatelessWidget {
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
  Widget build(BuildContext context) {
    return PadGrid(
      children: [
        for (var slot = 0; slot < 8; slot++) _slotPad(context.theme, slot),
      ],
    );
  }

  Widget _slotPad(MixarThemeData theme, int slot) {
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
                style: padActionStyle(theme).copyWith(
                  color: theme.colors.mutedForeground,
                  letterSpacing: 0,
                ),
              )
            : null,
      ),
    );

    final assign = onAssign;
    if (assign == null) {
      return pad;
    }
    return DropRegion(
      formats: const [Formats.plainText, Formats.fileUri],
      hitTestBehavior: HitTestBehavior.opaque,
      onDropOver: (event) {
        if (disabled) {
          return DropOperation.none;
        }
        for (final item in event.session.items) {
          if (parseTrackDragLocalData(item.localData) != null ||
              item.canProvide(Formats.plainText) ||
              item.canProvide(Formats.fileUri)) {
            return DropOperation.copy;
          }
        }
        return DropOperation.none;
      },
      onPerformDrop: (event) async {
        if (disabled) {
          return;
        }
        _performSamplerAssignDrop(event, slot, assign);
      },
      child: pad,
    );
  }
}

void _performSamplerAssignDrop(
  PerformDropEvent event,
  int slot,
  void Function(int slot, TrackDragPayload payload) assign,
) {
  var assigned = false;
  void tryAssign(TrackDragPayload? payload) {
    if (assigned || payload == null) {
      return;
    }
    assigned = true;
    assign(slot, payload);
  }

  for (final item in event.session.items) {
    tryAssign(parseTrackDragLocalData(item.localData));
    if (assigned) {
      return;
    }
  }

  for (final item in event.session.items) {
    final reader = item.dataReader;
    if (reader == null) {
      continue;
    }
    if (item.canProvide(Formats.plainText)) {
      reader.getValue<String>(
        Formats.plainText,
        (text) => tryAssign(parseTrackDragPlainText(text)),
      );
    }
    if (item.canProvide(Formats.fileUri)) {
      reader.getValue<Uri>(Formats.fileUri, (uri) {
        if (uri == null) {
          return;
        }
        final path = pathFromDroppedUri(uri);
        if (isSupportedAudioPath(path)) {
          tryAssign(payloadFromOsPath(path));
        }
      });
    }
  }
}
