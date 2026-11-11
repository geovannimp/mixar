import 'dart:async';

import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/models/pad_modes.dart';
import 'package:gui_flutter/core/utils/waveform_overlay_geometry.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/src/rust/api/event_mirrors.dart' show SavedLoop;
import 'package:gui_flutter/widgets/mixer/waveform/loop_band_hit_test.dart';
import 'package:gui_flutter/widgets/shell/mixar_context_menu.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_item.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:gui_flutter/widgets/shell/mixar_toast.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:material_ui/material_ui.dart';

/// Deletes a saved loop, exiting it first when it is the active loop.
Future<void> deleteSavedLoop({
  required BuildContext context,
  required WidgetRef ref,
  required int deckId,
  required String trackId,
  required SavedLoop loop,
}) async {
  try {
    final active = ref.read(deckActiveLoopProvider(deckId));
    final engine = await ref.read(engineTransportProvider.future);
    if (active != null &&
        active.inMs == loop.inMs &&
        active.outMs == loop.outMs) {
      await engine?.exitLoop(deckId: deckId);
    }
    final library = await ref.read(libraryTransportProvider.future);
    await library.deleteLoop(trackId: trackId, slot: loop.slot);
  } catch (e) {
    if (!context.mounted) {
      return;
    }
    showMixarToast(
      context: context,
      variant: MixarToastVariant.destructive,
      title: Text('$e'),
    );
  }
}

/// Shared saved-loop hit area for both waveform surfaces.
///
/// Right-click / long-press opens **Delete loop**; Shift+hover paints the band
/// red with a trash glyph and Shift+click deletes immediately. A plain tap is
/// forwarded to [onTapMs] when supplied.
class LoopBandDeleteRegion extends ConsumerStatefulWidget {
  const new({
    required this.deckId,
    required this.trackId,
    required this.loops,
    required this.originMs,
    required this.spanMs,
    required this.width,
    required this.height,
    required this.child,
    this.onTapMs,
    this.onMenuOpened,
    super.key,
  });

  final int deckId;
  final String trackId;
  final List<SavedLoop> loops;
  final int originMs;
  final int spanMs;
  final double width;
  final double height;
  final Widget child;
  final ValueChanged<int>? onTapMs;
  final VoidCallback? onMenuOpened;

  @override
  ConsumerState<LoopBandDeleteRegion> createState() =>
      _LoopBandDeleteRegionState();
}

class _LoopBandDeleteRegionState extends ConsumerState<LoopBandDeleteRegion> {
  SavedLoop? _menuLoop;
  SavedLoop? _hoverLoop;
  var _hoverX = 0.0;
  var _suppressTapUp = false;

  @override
  void initState() {
    super.initState();
    HardwareKeyboard.instance.addHandler(_onKey);
  }

  @override
  void dispose() {
    HardwareKeyboard.instance.removeHandler(_onKey);
    super.dispose();
  }

  bool _onKey(KeyEvent event) {
    if (event is KeyUpEvent &&
        (event.logicalKey == LogicalKeyboardKey.shiftLeft ||
            event.logicalKey == LogicalKeyboardKey.shiftRight)) {
      _clearHover();
    }
    return false;
  }

  SavedLoop? _loopAt(double x) {
    if (widget.trackId.isEmpty || widget.loops.isEmpty) {
      return null;
    }
    return savedLoopAtX(
      loops: widget.loops,
      x: x,
      width: widget.width,
      originMs: widget.originMs,
      spanMs: widget.spanMs,
    );
  }

  void _clearHover() {
    if (!mounted || _hoverLoop == null) {
      return;
    }
    setState(() => _hoverLoop = null);
  }

  void _updateHover(SavedLoop? loop, double x) {
    if (loop == null) {
      _clearHover();
      return;
    }
    if (_hoverLoop?.slot == loop.slot && (_hoverX - x).abs() < 0.5) {
      return;
    }
    setState(() {
      _hoverLoop = loop;
      _hoverX = x;
    });
  }

  Future<void> _delete(SavedLoop loop) async {
    await deleteSavedLoop(
      context: context,
      ref: ref,
      deckId: widget.deckId,
      trackId: widget.trackId,
      loop: loop,
    );
    _clearHover();
  }

  void _openMenu(
    MixarContextMenuHandle handle,
    double x,
    Offset globalPosition,
  ) {
    final loop = _loopAt(x);
    if (loop == null) {
      return;
    }
    widget.onMenuOpened?.call();
    setState(() => _menuLoop = loop);
    handle.showAt(globalPosition);
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final hover = _hoverLoop;
    final hoverRect = hover == null
        ? null
        : loopRegionRect(
            inMs: hover.inMs,
            outMs: hover.outMs,
            originMs: widget.originMs,
            spanMs: widget.spanMs,
            width: widget.width,
            height: widget.height,
          );
    final iconLeft = widget.width <= 18
        ? 0.0
        : (_hoverX - 9).clamp(0.0, widget.width - 18);

    return MixarContextMenu(
      enabled: widget.loops.isNotEmpty,
      menuBuilder: (context, handle) => MixarMenuItem(
        title: const Text('Delete loop'),
        destructive: true,
        onPress: () {
          handle.hide();
          final loop = _menuLoop;
          if (loop != null) {
            unawaited(_delete(loop));
          }
        },
      ),
      childBuilder: (context, handle) => MouseRegion(
        onHover: (event) {
          if (!shiftKeyPressed()) {
            _clearHover();
            return;
          }
          _updateHover(_loopAt(event.localPosition.dx), event.localPosition.dx);
        },
        onExit: (_) => _clearHover(),
        child: GestureDetector(
          behavior: HitTestBehavior.opaque,
          onTapDown: _hoverLoop == null
              ? null
              : (_) {
                  final loop = _hoverLoop;
                  if (loop == null) {
                    return;
                  }
                  _suppressTapUp = true;
                  _clearHover();
                  unawaited(_delete(loop));
                },
          onTapUp: widget.onTapMs == null
              ? null
              : (details) {
                  if (_suppressTapUp) {
                    _suppressTapUp = false;
                    return;
                  }
                  final onTapMs = widget.onTapMs;
                  if (onTapMs == null ||
                      widget.spanMs <= 0 ||
                      widget.width <= 0) {
                    return;
                  }
                  onTapMs(
                    waveformMsAtX(
                      x: details.localPosition.dx,
                      width: widget.width,
                      originMs: widget.originMs,
                      spanMs: widget.spanMs,
                    ),
                  );
                },
          onSecondaryTapDown: (details) => _openMenu(
            handle,
            details.localPosition.dx,
            details.globalPosition,
          ),
          onLongPressStart: (details) => _openMenu(
            handle,
            details.localPosition.dx,
            details.globalPosition,
          ),
          child: Stack(
            fit: StackFit.expand,
            children: [
              widget.child,
              if (hoverRect != null)
                Positioned(
                  left: hoverRect.left,
                  width: hoverRect.width,
                  top: 0,
                  bottom: 0,
                  child: IgnorePointer(
                    child: DecoratedBox(
                      decoration: BoxDecoration(
                        color: theme.colors.destructive.withValues(alpha: 0.18),
                        border: Border.all(color: theme.colors.destructive),
                      ),
                    ),
                  ),
                ),
              if (hover != null)
                Positioned(
                  left: iconLeft,
                  top: (widget.height - 18) / 2,
                  width: 18,
                  height: 18,
                  child: IgnorePointer(
                    child: Icon(
                      LucideIcons.trash2,
                      size: 18,
                      color: theme.colors.destructive,
                    ),
                  ),
                ),
            ],
          ),
        ),
      ),
    );
  }
}
