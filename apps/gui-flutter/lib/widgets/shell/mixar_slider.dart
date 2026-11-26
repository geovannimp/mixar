import 'dart:math' as math;

import 'package:flutter/gestures.dart' show kPrimaryButton;
import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:shadcn_ui/shadcn_ui.dart';

const _transparent = Color(0x00000000);

/// Horizontal inset applied to [ShadSlider] on either side when it paints
/// division marks.
///
/// [ShadSlider] hangs a 2x6 mark on every division boundary, centred on it, so
/// the first and last marks sit 1px past the track's ends. Drawn over the rail
/// they cut its stadium caps: the rounded end reads as a square block with a
/// notch. Insetting the track leaves the caps whole — the mark's outer edge
/// then starts 2px in, past the point where the 8px default track's 4px cap is
/// already 6px tall, the mark's own height.
const _divisionMarkInset = 3.0;

/// Gap between the rail and the [MixarSlider.readout] pill.
const _readoutGap = 12.0;

/// Room reserved inside the readout pill for the widest value.
///
/// The rail must not change width — and drag the thumb with it — as the digits
/// grow mid-drag. The default fits the app's readouts (`2048`, `100%`); a
/// label that is wider takes the space it needs, which is why a formatted
/// label such as `05 seconds` keeps its width across the whole range.
const _readoutMinWidth = 64.0;

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
    this.readout,
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

  /// Value shown in a pill to the right of the rail; `null` shows no pill.
  ///
  /// Format it at the call site (`512`, `50%`, a zero-padded `05 seconds`) and
  /// keep its length stable across the range: the pill does not chase the
  /// label, so a longer one would shorten the rail and move the thumb.
  final String? readout;

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
  bool _isOnThumb(
    Offset local,
    double width,
    double thumbRadius,
    double trackInset,
  ) {
    final range = widget.max - widget.min;
    final trackWidth = width - trackInset * 2;
    if (range <= 0 || trackWidth <= 0) {
      return false;
    }
    final t = ((_controller.value - widget.min) / range).clamp(0.0, 1.0);
    return (local.dx - (trackInset + t * trackWidth)).abs() <= thumbRadius;
  }

  /// Seek the same way [ShadSlider]'s `tapAndSlide` does.
  ///
  /// The double-tap recognizer holds the gesture arena for the first tap, so
  /// [ShadSlider]'s tap recognizer resolves a single click only after the
  /// double-tap timeout. This raw listener keeps the click-to-seek immediate
  /// instead of waiting out that window.
  void _seekTo(Offset local, double width, double trackInset) {
    final trackWidth = width - trackInset * 2;
    if (!widget.enabled || trackWidth <= 0) {
      return;
    }
    final t = ((local.dx - trackInset) / trackWidth).clamp(0.0, 1.0);
    final next = _snapValue(widget.min + t * (widget.max - widget.min));
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
    // Division marks are the only thing [ShadSlider] still paints; keep the
    // endpoints off the rail's caps (see [_divisionMarkInset]).
    final trackInset = (widget.divisions ?? 0) > 0 ? _divisionMarkInset : 0.0;
    final canReset = widget.enabled && widget.defaultValue != null;
    // [ShadSlider] paints a square-ended track and only rounds its active
    // fill's left edge. Paint the trail ourselves instead: one clip around the
    // full track height gives both ends a stadium cap at any value.
    final activeTrackColor = widget.enabled
        ? shad.sliderTheme.activeTrackColor ?? shad.colorScheme.primary
        : shad.sliderTheme.disabledActiveTrackColor ??
              shad.colorScheme.primary.withValues(alpha: 0.5);
    final inactiveTrackColor = widget.enabled
        ? shad.sliderTheme.inactiveTrackColor ?? shad.colorScheme.secondary
        : shad.sliderTheme.disabledInactiveTrackColor ??
              shad.colorScheme.secondary.withValues(alpha: 0.5);
    // ShadSlider paints the thumb outside its short track box, so the whole
    // reserved height must be the hit target.
    final rail = SizedBox(
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
                final onThumb = _isOnThumb(
                  local,
                  width,
                  thumbRadius,
                  trackInset,
                );
                _previousPressOnThumb = _lastPressOnThumb;
                _lastPressOnThumb = onThumb;
                final onTrack =
                    local.dy >= trackTop && local.dy <= trackTop + trackHeight;
                if (event.buttons == kPrimaryButton && !onThumb && onTrack) {
                  _seekTo(local, width, trackInset);
                }
              },
              child: Center(
                child: SizedBox(
                  height: trackHeight,
                  // The thumb and its focus ring paint outside the track box.
                  child: Stack(
                    clipBehavior: Clip.none,
                    children: [
                      Positioned.fill(
                        child: IgnorePointer(
                          child: ValueListenableBuilder<double>(
                            valueListenable: _controller,
                            builder: (context, value, _) => _MixarSliderTrack(
                              value: value,
                              min: widget.min,
                              max: widget.max,
                              activeColor: activeTrackColor,
                              inactiveColor: inactiveTrackColor,
                            ),
                          ),
                        ),
                      ),
                      // The rail (trail) keeps the full width; only the track
                      // ShadSlider measures and marks is inset, so its endpoint
                      // division marks land inside the stadium caps.
                      Padding(
                        padding: EdgeInsets.symmetric(horizontal: trackInset),
                        child: ShadSlider(
                          controller: _controller,
                          min: widget.min,
                          max: widget.max,
                          divisions: widget.divisions,
                          enabled: widget.enabled,
                          allowedInteraction: ShadSliderInteraction.slideOnly,
                          trackHeight: widget.trackHeight,
                          thumbRadius: widget.thumbRadius,
                          // The custom trail below supplies the paint; keep
                          // Shad's own track as the transparent hit area.
                          activeTrackColor: _transparent,
                          inactiveTrackColor: _transparent,
                          disabledActiveTrackColor: _transparent,
                          disabledInactiveTrackColor: _transparent,
                          onChanged: widget.onChanged,
                          onChangeStart: (_) => _dragging = true,
                          onChangeEnd: (_) => _dragging = false,
                          semanticFormatterCallback:
                              widget.semanticFormatterCallback,
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ),
          );
        },
      ),
    );
    final readout = widget.readout;
    if (readout == null) {
      return rail;
    }
    return Row(
      spacing: _readoutGap,
      children: [
        Expanded(child: rail),
        _MixarSliderReadout(text: readout),
      ],
    );
  }
}

/// Rounded trail painted under [ShadSlider]'s transparent track.
///
/// Clipping the full-height track gives both ends (and the active fill at the
/// extremes) a stadium cap; the value boundary stays square where the thumb
/// covers it.
class _MixarSliderTrack extends StatelessWidget {
  const new({
    required this.value,
    required this.min,
    required this.max,
    required this.activeColor,
    required this.inactiveColor,
  });

  final double value;
  final double min;
  final double max;
  final Color activeColor;
  final Color inactiveColor;

  @override
  Widget build(BuildContext context) {
    final range = max - min;
    final t = range == 0 ? 0.0 : ((value - min) / range).clamp(0.0, 1.0);
    return LayoutBuilder(
      builder: (context, constraints) {
        return ClipRRect(
          borderRadius: BorderRadius.circular(constraints.maxHeight / 2),
          child: Stack(
            fit: StackFit.expand,
            children: [
              ColoredBox(color: inactiveColor),
              Align(
                alignment: Alignment.centerLeft,
                child: FractionallySizedBox(
                  widthFactor: t,
                  heightFactor: 1,
                  child: ColoredBox(color: activeColor),
                ),
              ),
            ],
          ),
        );
      },
    );
  }
}

/// The readout pill beside the rail: one line, tabular, room reserved.
class _MixarSliderReadout extends StatelessWidget {
  const new({required this.text});

  final String text;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: theme.colors.card,
        border: Border.all(color: theme.colors.border),
        borderRadius: theme.style.borderRadius.pill,
      ),
      child: ConstrainedBox(
        constraints: const BoxConstraints(minWidth: _readoutMinWidth),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
          child: Text(
            text,
            maxLines: 1,
            textAlign: TextAlign.center,
            style: theme.typography.body.sm.copyWith(
              fontWeight: FontWeight.w600,
              fontFeatures: const [FontFeature.tabularFigures()],
            ),
          ),
        ),
      ),
    );
  }
}
