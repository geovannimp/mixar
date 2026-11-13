import 'dart:math' as math;

import 'package:flutter/gestures.dart' show kPrimaryButton;
import 'package:flutter/widgets.dart';
import 'package:shadcn_ui/shadcn_ui.dart';

/// Mixar slider over [ShadSlider].
///
/// [ShadSlider] reads `initialValue` only when its state is created, so a
/// value that arrives from outside (an engine echo, e.g. the DDJ-400 LEVEL
/// knob) never moves the thumb and the control looks dead. Own the controller
/// instead and mirror [value] into it whenever the widget rebuilds.
///
/// [ShadSlider]'s own box is exactly the track and it draws the thumb centred
/// on it, so a thumb taller than the track (plus its focus ring) paints outside
/// its layout bounds. Anything that clips that overflow (a scroll viewport, a
/// sized row, a cached paint layer) then crops the thumb. This widget reserves
/// the thumb's height and centres the track inside it, so nothing is painted
/// outside its box.
class MixarSlider extends StatefulWidget {
  const new({
    required this.value,
    required this.onChanged,
    super.key,
    this.min = 0,
    this.max = 1,
    this.divisions,
    this.enabled = true,
    this.defaultValue,
    this.semanticFormatterCallback,
    this.trackHeight,
    this.thumbRadius,
  });

  final double value;
  final ValueChanged<double> onChanged;
  final double min;
  final double max;
  final int? divisions;
  final bool enabled;

  /// Value a double click/tap on the thumb returns to. `null` disables the
  /// reset.
  final double? defaultValue;

  final String Function(double value)? semanticFormatterCallback;

  /// Track thickness; `null` uses the theme default (8).
  final double? trackHeight;

  /// Thumb radius; `null` uses the theme default (10). Compact rows pass
  /// smaller values so the thumb stops dominating the strip.
  final double? thumbRadius;

  @override
  State<MixarSlider> createState() => _MixarSliderState();
}

class _MixarSliderState extends State<MixarSlider> {
  late final ShadSliderController _controller = ShadSliderController(
    initialValue: _clampedValue,
  );

  /// True between [ShadSlider]'s drag start and end.
  bool _dragging = false;

  /// The last two presses, so a double tap resets only when both land on the
  /// thumb.
  bool _previousPressOnThumb = false;
  bool _lastPressOnThumb = false;
  bool _doubleTapOnThumb = false;

  /// [ShadSlider]'s own fallbacks for the track height and the thumb radius.
  static const _fallbackTrackHeight = 8.0;
  static const _fallbackThumbRadius = 10.0;

  /// Room ShadSlider's focused thumb adds around the thumb (2px ring border
  /// plus 2px padding on each side). Reserved whether or not the slider is
  /// focused, so gaining focus never repaints outside the box.
  static const _focusRingRoom = 8.0;

  double get _clampedValue => widget.value.clamp(widget.min, widget.max);

  void _resetToDefault() {
    final target = widget.defaultValue;
    if (target == null || !widget.enabled) {
      return;
    }
    final next = _snapValue(target);
    if (_controller.value != next) {
      _controller.value = next;
      widget.onChanged(next);
    }
  }

  /// Clamp to the range and snap to [MixarSlider.divisions], mirroring
  /// [ShadSlider]'s own value update.
  double _snapValue(double value) {
    final range = widget.max - widget.min;
    var next = value.clamp(widget.min, widget.max);
    final divisions = widget.divisions;
    if (divisions != null && divisions > 0 && range > 0) {
      final step = range / divisions;
      next = ((next - widget.min) / step).round() * step + widget.min;
      next = next.clamp(widget.min, widget.max);
    }
    return next;
  }

  /// True when [local] lands on the thumb. Seeking there on pointer-down
  /// would jump the value before the thumb's own absolute pan starts.
  bool _isOnThumb(Offset local, double width, double thumbRadius) {
    final range = widget.max - widget.min;
    if (range <= 0 || width <= 0) {
      return false;
    }
    final t = ((_controller.value - widget.min) / range).clamp(0.0, 1.0);
    return (local.dx - t * width).abs() <= thumbRadius;
  }

  /// Seek the same way [ShadSlider]'s `tapAndSlide` does.
  ///
  /// The double-tap recognizer holds the gesture arena for the first tap, so
  /// [ShadSlider]'s tap recognizer resolves a single click only after the
  /// double-tap timeout. This raw listener keeps the click-to-seek immediate
  /// instead of waiting out that window.
  void _seekTo(Offset local, double width) {
    if (!widget.enabled || width <= 0) {
      return;
    }
    final next = _snapValue(
      widget.min + (local.dx / width) * (widget.max - widget.min),
    );
    if (_controller.value != next) {
      _controller.value = next;
      widget.onChanged(next);
    }
  }

  @override
  void didUpdateWidget(covariant MixarSlider oldWidget) {
    super.didUpdateWidget(oldWidget);
    final external =
        oldWidget.value != widget.value ||
        oldWidget.min != widget.min ||
        oldWidget.max != widget.max;
    // Mid-drag the model lags the finger, so only an external change may move
    // the thumb. Outside a drag the model is authoritative: mirror it even when
    // its value did not change, which reconciles the drift a command leaves
    // behind when the engine ignores it (soft takeover, failed publish).
    if (!external && _dragging) {
      return;
    }
    final next = _clampedValue;
    if (_controller.value != next) {
      _controller.value = next;
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final shad = ShadTheme.of(context);
    final trackHeight =
        widget.trackHeight ??
        shad.sliderTheme.trackHeight ??
        _fallbackTrackHeight;
    final thumbRadius =
        widget.thumbRadius ??
        shad.sliderTheme.thumbRadius ??
        _fallbackThumbRadius;
    final hitHeight = math.max(trackHeight, thumbRadius * 2 + _focusRingRoom);
    final canReset = widget.enabled && widget.defaultValue != null;
    // ShadSlider paints the thumb outside its short track box, so the whole
    // reserved height must be the hit target.
    return SizedBox(
      // Constrained parents (a fixed-height row) keep their height; the track
      // then centres in it exactly as before.
      height: hitHeight,
      child: LayoutBuilder(
        builder: (context, constraints) {
          final trackTop = (constraints.maxHeight - trackHeight) / 2;
          return GestureDetector(
            behavior: HitTestBehavior.opaque,
            excludeFromSemantics: true,
            onDoubleTapDown: canReset
                ? (_) {
                    _doubleTapOnThumb =
                        _previousPressOnThumb && _lastPressOnThumb;
                  }
                : null,
            onDoubleTap: canReset
                ? () {
                    if (_doubleTapOnThumb) {
                      _doubleTapOnThumb = false;
                      _resetToDefault();
                    }
                  }
                : null,
            onDoubleTapCancel: canReset
                ? () => _doubleTapOnThumb = false
                : null,
            child: Listener(
              behavior: HitTestBehavior.opaque,
              onPointerDown: (event) {
                final width = constraints.maxWidth;
                final local = event.localPosition;
                final onThumb = _isOnThumb(local, width, thumbRadius);
                _previousPressOnThumb = _lastPressOnThumb;
                _lastPressOnThumb = onThumb;
                final onTrack =
                    local.dy >= trackTop && local.dy <= trackTop + trackHeight;
                if (event.buttons == kPrimaryButton && !onThumb && onTrack) {
                  _seekTo(local, width);
                }
              },
              child: Center(
                child: SizedBox(
                  height: trackHeight,
                  child: ShadSlider(
                    controller: _controller,
                    min: widget.min,
                    max: widget.max,
                    divisions: widget.divisions,
                    enabled: widget.enabled,
                    allowedInteraction: ShadSliderInteraction.slideOnly,
                    trackHeight: widget.trackHeight,
                    thumbRadius: widget.thumbRadius,
                    onChanged: widget.onChanged,
                    onChangeStart: (_) => _dragging = true,
                    onChangeEnd: (_) => _dragging = false,
                    semanticFormatterCallback: widget.semanticFormatterCallback,
                  ),
                ),
              ),
            ),
          );
        },
      ),
    );
  }
}
