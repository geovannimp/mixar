import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/m_tappable.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Tone of a toast. [primary] is the neutral notice; [success] and [warning]
/// colour the rail and glyph without tinting the copy.
enum MixarToastVariant { primary, success, warning, destructive }

const _toastGap = 8.0;
const _toastMinWidth = 300.0;
const _toastMaxWidth = 380.0;
const _toastEnter = Duration(milliseconds: 320);
const _toastExit = Duration(milliseconds: 180);
const _toastLift = 18.0;
const _toastRight = 20.0;
const _toastBottom = 24.0;

var _mixarToastSeq = 0;

/// Mixar toast pinned to the bottom-right corner, rising from the bottom edge.
///
/// Toasts stack upward, newest nearest the corner, and each one can be
/// dismissed by hand, by its own timer (paused on hover), or by [id] through
/// [dismissMixarToast].
void showMixarToast({
  required BuildContext context,
  required Widget title,
  MixarToastVariant variant = MixarToastVariant.primary,
  Widget? description,
  Widget Function(BuildContext context, VoidCallback dismiss)? suffixBuilder,
  Duration? duration = const Duration(seconds: 5),
  VoidCallback? onDismiss,
  String? id,
}) {
  final toastId = id ?? 'mixar-toast-${_mixarToastSeq++}';
  // Duration.zero = persistent (controller offer). Null maps the same way.
  final showFor = duration == null || duration <= Duration.zero
      ? null
      : duration;

  // Build the host first: a fresh app tree clears toasts left by the old one.
  _ensureToastHost(context);
  _toasts.add(
    _MixarToastSpec(
      id: toastId,
      variant: variant,
      title: title,
      description: description,
      suffixBuilder: suffixBuilder,
      duration: showFor,
      onDismiss: onDismiss,
    ),
  );
}

/// Close the toast shown with [id] (and animate it out).
///
/// No-op when nothing matches or [context] is already unmounted.
void dismissMixarToast(BuildContext context, {required String id}) {
  if (!context.mounted) {
    return;
  }
  _toasts.dismiss(id);
}

// ---------------------------------------------------------------------------
// Host
// ---------------------------------------------------------------------------

final _toasts = _MixarToastQueue();

OverlayEntry? _hostEntry;
OverlayState? _hostOverlay;

/// Insert the singleton host into the root overlay when it is missing.
///
/// The host sits above every route (dialogs included) so a toast raised from a
/// dialog does not disappear with it.
void _ensureToastHost(BuildContext context) {
  final overlay = _hostOverlay;
  if (overlay != null && overlay.mounted && _hostEntry != null) {
    return;
  }
  _hostEntry = null;
  _hostOverlay = null;
  // A dead overlay means a new app tree; do not carry stale toasts into it.
  _toasts.clear();

  final target = Overlay.maybeOf(context, rootOverlay: true);
  if (target == null) {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (context.mounted) {
        _ensureToastHost(context);
      }
    });
    return;
  }

  final entry = OverlayEntry(builder: (_) => _MixarToastHost(queue: _toasts));
  _hostOverlay = target;
  _hostEntry = entry;
  target.insert(entry);
}

class _MixarToastHost extends StatelessWidget {
  const new({required this.queue});

  final _MixarToastQueue queue;

  @override
  Widget build(BuildContext context) {
    final insets = MediaQuery.paddingOf(context);
    return Stack(
      children: [
        Positioned(
          right: _toastRight + insets.right,
          bottom: _toastBottom + insets.bottom,
          child: AnimatedBuilder(
            animation: queue,
            builder: (context, _) {
              return Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.end,
                children: [
                  for (final spec in queue.specs)
                    _MixarToastSlot(
                      key: ValueKey(spec.id),
                      spec: spec,
                      queue: queue,
                    ),
                ],
              );
            },
          ),
        ),
      ],
    );
  }
}

// ---------------------------------------------------------------------------
// Queue
// ---------------------------------------------------------------------------

class _MixarToastSpec {
  new({
    required this.id,
    required this.variant,
    required this.title,
    required this.description,
    required this.suffixBuilder,
    required this.duration,
    required this.onDismiss,
  });

  final String id;
  final MixarToastVariant variant;
  final Widget title;
  final Widget? description;
  final Widget Function(BuildContext context, VoidCallback dismiss)?
  suffixBuilder;

  /// Null means the toast stays until dismissed by hand.
  final Duration? duration;
  final VoidCallback? onDismiss;

  /// Flipped by the timer, the close button, or [dismissMixarToast].
  final ValueNotifier<bool> closing = ValueNotifier(false);
}

class _MixarToastQueue extends ChangeNotifier {
  final _specs = <_MixarToastSpec>[];

  List<_MixarToastSpec> get specs => List.unmodifiable(_specs);

  void add(_MixarToastSpec spec) {
    final index = _specs.indexWhere((s) => s.id == spec.id);
    if (index >= 0) {
      _specs[index] = spec;
    } else {
      _specs.add(spec);
    }
    notifyListeners();
  }

  void dismiss(String id) {
    for (final spec in _specs) {
      if (spec.id == id) {
        spec.closing.value = true;
      }
    }
  }

  void remove(_MixarToastSpec spec) {
    if (_specs.remove(spec)) {
      notifyListeners();
    }
  }

  void clear() {
    if (_specs.isEmpty) {
      return;
    }
    _specs.clear();
    notifyListeners();
  }
}

// ---------------------------------------------------------------------------
// Motion
// ---------------------------------------------------------------------------

/// Damped spring for the rise. Starts at 0, overshoots ~8%, settles at 1.
class _ToastSpringCurve extends Curve {
  const new();

  @override
  double transformInternal(double t) {
    if (t >= 1) {
      return 1;
    }
    const damping = 7.5;
    const frequency = 9.0;
    return 1 - math.exp(-damping * t) * math.cos(frequency * t);
  }
}

class _MixarToastSlot extends StatefulWidget {
  const new({required this.spec, required this.queue, super.key});

  final _MixarToastSpec spec;
  final _MixarToastQueue queue;

  @override
  State<_MixarToastSlot> createState() => _MixarToastSlotState();
}

class _MixarToastSlotState extends State<_MixarToastSlot>
    with TickerProviderStateMixin {
  late final AnimationController _enter;
  late final Animation<double> _slide;
  late final Animation<double> _fade;
  late final Animation<double> _size;

  final _clock = Stopwatch();
  Timer? _autoClose;
  Duration _remaining = Duration.zero;
  var _started = false;
  var _reduceMotion = false;
  var _closing = false;
  var _finished = false;

  @override
  void initState() {
    super.initState();
    _enter = AnimationController(
      vsync: this,
      duration: _toastEnter,
      reverseDuration: _toastExit,
    )..addStatusListener(_onStatus);
    // Overshoot only the paint transform: [SizeTransition] must stay <= 1.
    _slide = CurvedAnimation(
      parent: _enter,
      curve: const _ToastSpringCurve(),
      reverseCurve: Curves.easeInCubic,
    );
    _fade = CurvedAnimation(
      parent: _enter,
      curve: const Interval(0, 0.45, curve: Curves.easeOut),
      reverseCurve: const Interval(0.55, 1, curve: Curves.easeIn),
    );
    _size = CurvedAnimation(
      parent: _enter,
      curve: Curves.easeOutCubic,
      reverseCurve: Curves.easeInCubic,
    );
    widget.spec.closing.addListener(_onClosing);
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (_started) {
      return;
    }
    _started = true;
    _reduceMotion = MediaQuery.disableAnimationsOf(context);
    if (_reduceMotion) {
      _enter.value = 1;
    } else {
      _enter.forward();
    }
    _armAutoClose(widget.spec.duration);
  }

  @override
  void didUpdateWidget(_MixarToastSlot oldWidget) {
    super.didUpdateWidget(oldWidget);
    final oldSpec = oldWidget.spec;
    if (identical(oldSpec, widget.spec)) {
      return;
    }
    oldSpec.closing.removeListener(_onClosing);
    widget.spec.closing.addListener(_onClosing);
    if (widget.spec.closing.value) {
      _close();
      return;
    }
    // A repeated id restarts the toast rather than overlaps the old one.
    _stopAutoClose();
    _closing = false;
    _finished = false;
    if (!_reduceMotion) {
      _enter.forward(from: 0);
    }
    _armAutoClose(widget.spec.duration);
  }

  @override
  void dispose() {
    _stopAutoClose();
    widget.spec.closing.removeListener(_onClosing);
    _enter.dispose();
    super.dispose();
  }

  void _armAutoClose(Duration? duration) {
    if (duration == null) {
      return;
    }
    _remaining = duration;
    _clock
      ..reset()
      ..start();
    _autoClose = Timer(_remaining, _close);
  }

  void _stopAutoClose() {
    _autoClose?.cancel();
    _autoClose = null;
    _clock.stop();
  }

  void _onClosing() {
    if (widget.spec.closing.value) {
      _close();
    }
  }

  void _onStatus(AnimationStatus status) {
    if (status == AnimationStatus.dismissed && _closing) {
      _finish();
    }
  }

  void _close() {
    if (_closing || _finished) {
      return;
    }
    _closing = true;
    _stopAutoClose();
    if (_reduceMotion || _enter.value == 0) {
      _finish();
      return;
    }
    _enter.reverse();
  }

  void _finish() {
    if (_finished) {
      return;
    }
    _finished = true;
    _stopAutoClose();
    widget.spec.onDismiss?.call();
    widget.queue.remove(widget.spec);
  }

  /// Hovering holds the toast on screen instead of counting it down.
  void _pause() {
    final timer = _autoClose;
    if (timer == null) {
      return;
    }
    timer.cancel();
    _autoClose = null;
    _clock.stop();
    _remaining -= _clock.elapsed;
    if (_remaining < Duration.zero) {
      _remaining = Duration.zero;
    }
    _clock.reset();
  }

  void _resume() {
    if (_closing || _autoClose != null || widget.spec.duration == null) {
      return;
    }
    if (_remaining <= Duration.zero) {
      _close();
      return;
    }
    _clock
      ..reset()
      ..start();
    _autoClose = Timer(_remaining, _close);
  }

  @override
  Widget build(BuildContext context) {
    final card = _MixarToastCard(
      spec: widget.spec,
      dismiss: () => widget.spec.closing.value = true,
    );

    return AnimatedBuilder(
      animation: _enter,
      builder: (context, child) {
        final slide = _slide.value;
        final fade = _reduceMotion ? 1.0 : _fade.value;
        return Opacity(
          opacity: fade,
          child: Transform.translate(
            offset: Offset(0, (1 - slide) * _toastLift),
            child: Transform.scale(
              scale: 1 - (1 - slide) * 0.02,
              alignment: Alignment.bottomRight,
              child: child,
            ),
          ),
        );
      },
      child: SizeTransition(
        alignment: Alignment.bottomCenter,
        sizeFactor: _size,
        child: Padding(
          padding: const EdgeInsets.only(top: _toastGap),
          child: MouseRegion(
            onEnter: (_) => _pause(),
            onExit: (_) => _resume(),
            child: Semantics(container: true, liveRegion: true, child: card),
          ),
        ),
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// Chrome
// ---------------------------------------------------------------------------

@immutable
class _MixarToastCard extends StatelessWidget {
  const new({required this.spec, required this.dismiss});

  final _MixarToastSpec spec;
  final VoidCallback dismiss;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final colors = theme.colors;
    final (accent, glyph, tintAlpha) = switch (spec.variant) {
      MixarToastVariant.primary => (colors.primary, LucideIcons.info, 0.0),
      MixarToastVariant.success => (colors.primary, LucideIcons.check, 0.05),
      MixarToastVariant.warning => (
        colors.cueOn,
        LucideIcons.triangleAlert,
        0.06,
      ),
      MixarToastVariant.destructive => (
        colors.destructive,
        LucideIcons.circleAlert,
        0.06,
      ),
    };
    final description = spec.description;
    final suffix = spec.suffixBuilder?.call(context, dismiss);

    return IntrinsicWidth(
      child: ConstrainedBox(
        constraints: const BoxConstraints(
          minWidth: _toastMinWidth,
          maxWidth: _toastMaxWidth,
        ),
        child: DecoratedBox(
          decoration: BoxDecoration(
            color: Color.alphaBlend(
              accent.withValues(alpha: tintAlpha),
              colors.card,
            ),
            border: Border(
              left: BorderSide(color: accent, width: 3),
              top: BorderSide(
                color: colors.border,
                width: theme.style.borderWidth,
              ),
              right: BorderSide(
                color: colors.border,
                width: theme.style.borderWidth,
              ),
              bottom: BorderSide(
                color: colors.border,
                width: theme.style.borderWidth,
              ),
            ),
          ),
          child: Padding(
            padding: const EdgeInsets.fromLTRB(14, 13, 8, 13),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Row(
                  children: [
                    // Outfit's 14px glyph sits ~1.5px below its line-box
                    // centre; nudge the glyph so it reads level with the title.
                    Transform.translate(
                      offset: const Offset(0, 1.5),
                      child: Icon(glyph, size: 16, color: accent),
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      child: DefaultTextStyle(
                        style: theme.typography.body.sm.copyWith(
                          color: colors.foreground,
                          fontWeight: FontWeight.w600,
                        ),
                        child: spec.title,
                      ),
                    ),
                    const SizedBox(width: 6),
                    _ToastCloseButton(onPress: dismiss),
                  ],
                ),
                if (description != null) ...[
                  const SizedBox(height: 4),
                  Padding(
                    padding: const EdgeInsets.only(left: 28),
                    child: DefaultTextStyle(
                      style: theme.typography.body.xs.copyWith(
                        color: colors.mutedForeground,
                        height: 1.4,
                      ),
                      child: description,
                    ),
                  ),
                ],
                if (suffix != null) ...[
                  const SizedBox(height: 10),
                  Align(alignment: Alignment.centerRight, child: suffix),
                ],
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _ToastCloseButton extends StatelessWidget {
  const new({required this.onPress});

  final VoidCallback onPress;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final colors = theme.colors;
    return MTappable(
      semanticsLabel: 'Dismiss notification',
      onPress: onPress,
      builder: (context, state) {
        return SizedBox(
          width: 26,
          height: 26,
          child: ColoredBox(
            color: state.hovered ? theme.hoverWash : const Color(0x00000000),
            child: Center(
              child: Icon(
                LucideIcons.x,
                size: 14,
                color: state.hovered
                    ? colors.foreground
                    : colors.mutedForeground,
              ),
            ),
          ),
        );
      },
    );
  }
}
