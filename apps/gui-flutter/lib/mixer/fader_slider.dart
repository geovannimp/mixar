import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';

/// Soft snap near mid — narrow so centering is optional (Tauri `CENTER_SNAP_THRESHOLD`).
const kFaderCenterSnapThreshold = 0.8;

enum FaderOrientation { vertical, horizontal }

enum FaderAccent { a, b, neutral }

enum _TickSize { major, minor }

class _FaderTick {
  const _FaderTick(this.pos, this.size);
  final double pos;
  final _TickSize size;
}

/// Hierarchical ticks every 10%: major at ends; mid is minor unless [centerNotch].
const _faderTicks = <_FaderTick>[
  _FaderTick(0, _TickSize.major),
  _FaderTick(10, _TickSize.minor),
  _FaderTick(20, _TickSize.minor),
  _FaderTick(30, _TickSize.minor),
  _FaderTick(40, _TickSize.minor),
  _FaderTick(50, _TickSize.minor),
  _FaderTick(60, _TickSize.minor),
  _FaderTick(70, _TickSize.minor),
  _FaderTick(80, _TickSize.minor),
  _FaderTick(90, _TickSize.minor),
  _FaderTick(100, _TickSize.major),
];

const _tickLength = {_TickSize.major: 10.0, _TickSize.minor: 6.0};

/// Deck A/B / neutral tokens matching Tauri `DECK_ACCENTS` / `NEUTRAL_FADER_TRACK`.
class FaderColors {
  const FaderColors({
    required this.track,
    required this.indicator,
    required this.grip,
  });

  final Color track;
  final Color indicator;
  final Color grip;

  static const a = FaderColors(
    track: Color(0x140ea5e9), // sky-500 @ 8%
    indicator: Color(0x8c38bdf8), // sky-400 @ 55%
    grip: Color(0xff38bdf8), // sky-400
  );

  static const b = FaderColors(
    track: Color(0x14f43f5e), // rose-500 @ 8%
    indicator: Color(0x8cfb7185), // rose-400 @ 55%
    grip: Color(0xfffb7185), // rose-400
  );

  static const neutral = FaderColors(
    track: Color(0x1f71717a), // zinc-500 @ 12%
    indicator: Color(0x80a1a1aa), // zinc-400 @ 50%
    grip: Color(0x8071717a), // zinc-500 @ 50%
  );

  static FaderColors forAccent(FaderAccent accent) => switch (accent) {
    FaderAccent.a => a,
    FaderAccent.b => b,
    FaderAccent.neutral => neutral,
  };
}

FaderAccent? faderAccentForDeck(int deckId) => switch (deckId) {
  0 => FaderAccent.a,
  1 => FaderAccent.b,
  _ => null,
};

String deckDisplayLabel(int deckId) => switch (deckId) {
  0 => 'Deck A',
  1 => 'Deck B',
  _ => 'Deck ${deckId + 1}',
};

void _requireValidRange(double min, double max) {
  if (!(min.isFinite && max.isFinite && min < max)) {
    throw ArgumentError.value(
      (min: min, max: max),
      'min/max',
      'expected finite min < max',
    );
  }
}

double snapFaderToStep(double value, double step, {double origin = 0.0}) {
  if (step <= 0) {
    return value;
  }
  final snapped = origin + ((value - origin) / step).round() * step;
  return snapped == 0 ? 0.0 : snapped;
}

/// Soft-snap toward mid when [centerNotch] is enabled (Tauri parity on 0–100 scale).
double snapTowardCenter(
  double value,
  double min,
  double max, {
  double thresholdAtHundred = kFaderCenterSnapThreshold,
}) {
  _requireValidRange(min, max);
  final mid = (min + max) / 2;
  final threshold = thresholdAtHundred * (max - min) / 100;
  return (value - mid).abs() <= threshold ? mid : value;
}

/// Painted thumb sizes — keep in sync with [_FaderPainter].
const kFaderThumbV = Size(20, 10);
const kFaderThumbH = Size(10, 16);

/// Extra hit padding so the knob stays easy to grab near the track.
const kFaderThumbHitPadding = 4.0;

double _normalizeFaderT(double value, double min, double max) {
  _requireValidRange(min, max);
  return ((value - min) / (max - min)).clamp(0.0, 1.0);
}

double _finishFaderValue(
  double raw, {
  required double min,
  required double max,
  required double step,
  required bool centerNotch,
}) {
  var next = snapFaderToStep(raw, step, origin: min).clamp(min, max).toDouble();
  if (centerNotch) {
    next = snapTowardCenter(next, min, max);
  }
  return next;
}

/// Thumb extent along the travel axis (height for vertical, width for horizontal).
double faderThumbExtentAlongAxis(FaderOrientation orientation) =>
    switch (orientation) {
      FaderOrientation.vertical => kFaderThumbV.height,
      FaderOrientation.horizontal => kFaderThumbH.width,
    };

/// Thumb-center travel length — insets by half thumb so ends stay inside [size].
double faderTravelLength(Size size, FaderOrientation orientation) {
  final length = switch (orientation) {
    FaderOrientation.vertical => size.height,
    FaderOrientation.horizontal => size.width,
  };
  final travel = length - faderThumbExtentAlongAxis(orientation);
  return travel > 0 ? travel : 0.0;
}

Offset faderThumbCenter({
  required Size size,
  required FaderOrientation orientation,
  required double t,
}) {
  final half = faderThumbExtentAlongAxis(orientation) / 2;
  final travel = faderTravelLength(size, orientation);
  return switch (orientation) {
    FaderOrientation.vertical => Offset(
      size.width / 2,
      half + (1.0 - t) * travel,
    ),
    FaderOrientation.horizontal => Offset(half + t * travel, size.height / 2),
  };
}

/// Painted thumb rect for normalized [t] (0 = min end, 1 = max end).
Rect faderThumbRect({
  required Size size,
  required FaderOrientation orientation,
  required double t,
}) {
  final thumbSize = switch (orientation) {
    FaderOrientation.vertical => kFaderThumbV,
    FaderOrientation.horizontal => kFaderThumbH,
  };
  final thumbCenter = faderThumbCenter(
    size: size,
    orientation: orientation,
    t: t,
  );
  return Rect.fromCenter(
    center: thumbCenter,
    width: thumbSize.width,
    height: thumbSize.height,
  );
}

/// Hit target for the thumb, including slight inflate past the painted knob.
Rect faderThumbHitRect({
  required Size size,
  required FaderOrientation orientation,
  required double t,
}) {
  return faderThumbRect(
    size: size,
    orientation: orientation,
    t: t,
  ).inflate(kFaderThumbHitPadding);
}

/// Map pointer local offset → value. Vertical: max at top; horizontal: min at left.
///
/// Uses the same inset travel as the painted thumb so end positions remain
/// hittable inside the layout box.
double valueFromFaderPointer({
  required Offset local,
  required Size size,
  required FaderOrientation orientation,
  required double min,
  required double max,
  required double step,
  required bool centerNotch,
}) {
  _requireValidRange(min, max);
  final half = faderThumbExtentAlongAxis(orientation) / 2;
  final travel = faderTravelLength(size, orientation);
  final t = travel <= 0
      ? 0.0
      : switch (orientation) {
          FaderOrientation.vertical =>
            (1.0 - ((local.dy - half) / travel)).clamp(0.0, 1.0),
          FaderOrientation.horizontal => ((local.dx - half) / travel).clamp(
            0.0,
            1.0,
          ),
        };
  return _finishFaderValue(
    min + t * (max - min),
    min: min,
    max: max,
    step: step,
    centerNotch: centerNotch,
  );
}

/// Relative drag along the fader axis from a thumb grab (no jump-to-pointer).
///
/// Vertical: decreasing [currentAxis] (up) raises value. Horizontal: increasing
/// [currentAxis] (right) raises value. [trackLength] is the painted travel span.
double valueFromFaderRelativeDrag({
  required double startValue,
  required double startAxis,
  required double currentAxis,
  required double trackLength,
  required FaderOrientation orientation,
  required double min,
  required double max,
  required double step,
  required bool centerNotch,
}) {
  _requireValidRange(min, max);
  if (trackLength <= 0) {
    return _finishFaderValue(
      startValue,
      min: min,
      max: max,
      step: step,
      centerNotch: centerNotch,
    );
  }
  final axisDelta = switch (orientation) {
    FaderOrientation.vertical => startAxis - currentAxis,
    FaderOrientation.horizontal => currentAxis - startAxis,
  };
  return _finishFaderValue(
    startValue + (axisDelta / trackLength) * (max - min),
    min: min,
    max: max,
    step: step,
    centerNotch: centerNotch,
  );
}

/// DJ-style fader: markers, optional center notch, deck accent grip (Tauri `Slider` fader).
class FaderSlider extends StatefulWidget {
  FaderSlider({
    required this.value,
    required this.onValueChange,
    this.min = 0,
    this.max = 100,
    this.step = 1,
    this.orientation = FaderOrientation.vertical,
    this.accent = FaderAccent.neutral,
    this.showIndicator = true,
    this.showMarkers = false,
    this.centerNotch = false,
    this.crossfaderTrack = false,
    this.disabled = false,
    this.semanticLabel,
    super.key,
  }) {
    _requireValidRange(min, max);
  }

  final double value;
  final ValueChanged<double> onValueChange;
  final double min;
  final double max;
  final double step;
  final FaderOrientation orientation;
  final FaderAccent accent;
  final bool showIndicator;
  final bool showMarkers;
  final bool centerNotch;
  final bool crossfaderTrack;
  final bool disabled;
  final String? semanticLabel;

  @override
  State<FaderSlider> createState() => _FaderSliderState();
}

class _FaderAdjustIntent extends Intent {
  const _FaderAdjustIntent(this.delta);
  final double delta;
}

class _FaderSliderState extends State<FaderSlider> {
  bool _dragging = false;
  bool _relative = false;
  double? _startValue;
  double? _startAxis;

  void _clearDrag() {
    setState(() {
      _dragging = false;
      _relative = false;
      _startValue = null;
      _startAxis = null;
    });
  }

  void _emitFromLocal(Offset local, Size size) {
    widget.onValueChange(
      valueFromFaderPointer(
        local: local,
        size: size,
        orientation: widget.orientation,
        min: widget.min,
        max: widget.max,
        step: widget.step,
        centerNotch: widget.centerNotch,
      ),
    );
  }

  void _emitRelative(Offset local, Size size) {
    final startValue = _startValue;
    final startAxis = _startAxis;
    if (startValue == null || startAxis == null) {
      return;
    }
    final trackLength = faderTravelLength(size, widget.orientation);
    final currentAxis = switch (widget.orientation) {
      FaderOrientation.vertical => local.dy,
      FaderOrientation.horizontal => local.dx,
    };
    widget.onValueChange(
      valueFromFaderRelativeDrag(
        startValue: startValue,
        startAxis: startAxis,
        currentAxis: currentAxis,
        trackLength: trackLength,
        orientation: widget.orientation,
        min: widget.min,
        max: widget.max,
        step: widget.step,
        centerNotch: widget.centerNotch,
      ),
    );
  }

  void _nudgeBy(double delta) {
    if (widget.disabled || delta == 0) {
      return;
    }
    widget.onValueChange(
      _finishFaderValue(
        (widget.value + delta).clamp(widget.min, widget.max).toDouble(),
        min: widget.min,
        max: widget.max,
        step: widget.step,
        centerNotch: widget.centerNotch,
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final colors = FaderColors.forAccent(widget.accent);
    final opacity = widget.disabled ? 0.45 : 1.0;
    final t = _normalizeFaderT(widget.value, widget.min, widget.max);
    final step = widget.step > 0 ? widget.step : 1.0;

    return Opacity(
      opacity: opacity,
      child: LayoutBuilder(
        builder: (context, constraints) {
          final size = Size(constraints.maxWidth, constraints.maxHeight);
          return Focus(
            child: Shortcuts(
              shortcuts: <ShortcutActivator, Intent>{
                const SingleActivator(LogicalKeyboardKey.arrowUp):
                    _FaderAdjustIntent(step),
                const SingleActivator(LogicalKeyboardKey.arrowDown):
                    _FaderAdjustIntent(-step),
                const SingleActivator(LogicalKeyboardKey.arrowRight):
                    _FaderAdjustIntent(step),
                const SingleActivator(LogicalKeyboardKey.arrowLeft):
                    _FaderAdjustIntent(-step),
              },
              child: Actions(
                actions: <Type, Action<Intent>>{
                  _FaderAdjustIntent: CallbackAction<_FaderAdjustIntent>(
                    onInvoke: (intent) {
                      _nudgeBy(intent.delta);
                      return null;
                    },
                  ),
                },
                child: Semantics(
                  label: widget.semanticLabel,
                  slider: true,
                  enabled: !widget.disabled,
                  value: widget.value.toStringAsFixed(2),
                  increasedValue: widget.disabled
                      ? null
                      : (widget.value + step)
                            .clamp(widget.min, widget.max)
                            .toStringAsFixed(2),
                  decreasedValue: widget.disabled
                      ? null
                      : (widget.value - step)
                            .clamp(widget.min, widget.max)
                            .toStringAsFixed(2),
                  onIncrease: widget.disabled ? null : () => _nudgeBy(step),
                  onDecrease: widget.disabled ? null : () => _nudgeBy(-step),
                  child: Listener(
                    behavior: HitTestBehavior.opaque,
                    onPointerDown: widget.disabled
                        ? null
                        : (event) {
                            final onThumb = faderThumbHitRect(
                              size: size,
                              orientation: widget.orientation,
                              t: t,
                            ).contains(event.localPosition);
                            setState(() {
                              _dragging = true;
                              _relative = onThumb;
                              if (onThumb) {
                                _startValue = widget.value;
                                _startAxis = switch (widget.orientation) {
                                  FaderOrientation.vertical =>
                                    event.localPosition.dy,
                                  FaderOrientation.horizontal =>
                                    event.localPosition.dx,
                                };
                              } else {
                                _startValue = null;
                                _startAxis = null;
                              }
                            });
                            if (!onThumb) {
                              _emitFromLocal(event.localPosition, size);
                            }
                          },
                    onPointerMove: widget.disabled
                        ? null
                        : (event) {
                            if (!_dragging) {
                              return;
                            }
                            if (_relative) {
                              _emitRelative(event.localPosition, size);
                            } else {
                              _emitFromLocal(event.localPosition, size);
                            }
                          },
                    onPointerUp: (_) => _clearDrag(),
                    onPointerCancel: (_) => _clearDrag(),
                    child: CustomPaint(
                      size: size,
                      painter: _FaderPainter(
                        t: t,
                        orientation: widget.orientation,
                        colors: colors,
                        showIndicator: widget.showIndicator,
                        showMarkers: widget.showMarkers,
                        centerNotch: widget.centerNotch,
                        crossfaderTrack: widget.crossfaderTrack,
                        dragging: _dragging,
                      ),
                    ),
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

class _FaderPainter extends CustomPainter {
  _FaderPainter({
    required this.t,
    required this.orientation,
    required this.colors,
    required this.showIndicator,
    required this.showMarkers,
    required this.centerNotch,
    required this.crossfaderTrack,
    required this.dragging,
  });

  final double t;
  final FaderOrientation orientation;
  final FaderColors colors;
  final bool showIndicator;
  final bool showMarkers;
  final bool centerNotch;
  final bool crossfaderTrack;
  final bool dragging;

  static const _trackThickness = 4.0;
  static const _laneExtend = 6.0; // 1.5 * 4
  static const _tickGap = 5.0;
  static const _tickThickness = 2.0;
  static const _thumbBorder = Color(0xa6a1a1aa); // zinc-400 @ 65%
  static const _thumbTop = Color(0xfad4d4d8); // zinc-300
  static const _thumbBottom = Color(0xfaa1a1aa); // zinc-400
  static const _tickTone = Color(0x3371717a); // zinc-500 @ 20%
  static const _tickEmphasize = Color(0x4071717a); // zinc-500 @ 25%
  static const _crossStart = Color(0x140ea5e9); // sky-500 @ 8%
  static const _crossMid = Color(0x1a71717a); // zinc-500 @ 10%
  static const _crossEnd = Color(0x14f43f5e); // rose-500 @ 8%

  @override
  void paint(Canvas canvas, Size size) {
    final vertical = orientation == FaderOrientation.vertical;
    final trackRect = vertical
        ? Rect.fromCenter(
            center: Offset(size.width / 2, size.height / 2),
            width: _trackThickness,
            height: size.height + _laneExtend * 2,
          )
        : Rect.fromCenter(
            center: Offset(size.width / 2, size.height / 2),
            width: size.width + _laneExtend * 2,
            height: _trackThickness,
          );

    final trackRRect = RRect.fromRectAndRadius(
      trackRect,
      const Radius.circular(999),
    );

    if (crossfaderTrack && !vertical) {
      final paint = Paint()
        ..shader = const LinearGradient(
          colors: [_crossStart, _crossMid, _crossEnd],
        ).createShader(trackRect);
      canvas.drawRRect(trackRRect, paint);
    } else {
      canvas.drawRRect(trackRRect, Paint()..color = colors.track);
    }

    if (showMarkers) {
      _paintMarkers(canvas, size, vertical);
    }

    if (showIndicator) {
      final thumbCenter = faderThumbCenter(
        size: size,
        orientation: orientation,
        t: t,
      );
      final indicator = vertical
          ? Rect.fromLTRB(
              trackRect.left,
              thumbCenter.dy,
              trackRect.right,
              trackRect.bottom,
            )
          : Rect.fromLTRB(
              trackRect.left,
              trackRect.top,
              thumbCenter.dx,
              trackRect.bottom,
            );
      canvas.drawRRect(
        RRect.fromRectAndCorners(
          indicator,
          topLeft: vertical ? Radius.zero : const Radius.circular(999),
          bottomLeft: const Radius.circular(999),
          bottomRight: vertical ? const Radius.circular(999) : Radius.zero,
          topRight: Radius.zero,
        ),
        Paint()..color = colors.indicator,
      );
    }

    final thumbRect = faderThumbRect(
      size: size,
      orientation: orientation,
      t: t,
    );
    final thumbCenter = thumbRect.center;

    canvas.save();
    if (dragging) {
      canvas.translate(thumbCenter.dx, thumbCenter.dy);
      canvas.scale(1.05);
      canvas.translate(-thumbCenter.dx, -thumbCenter.dy);
    }

    final thumbRRect = RRect.fromRectAndRadius(
      thumbRect,
      const Radius.circular(2),
    );
    canvas.drawRRect(
      thumbRRect,
      Paint()
        ..shader = LinearGradient(
          begin: Alignment.topCenter,
          end: Alignment.bottomCenter,
          colors: const [_thumbTop, _thumbBottom],
        ).createShader(thumbRect),
    );
    canvas.drawRRect(
      thumbRRect,
      Paint()
        ..color = _thumbBorder
        ..style = PaintingStyle.stroke
        ..strokeWidth = 1,
    );

    // Grip line (Tauri `after:` pseudo).
    if (vertical) {
      canvas.drawLine(
        Offset(thumbRect.left + 4, thumbCenter.dy),
        Offset(thumbRect.right - 4, thumbCenter.dy),
        Paint()
          ..color = colors.grip
          ..strokeWidth = 1
          ..strokeCap = StrokeCap.round,
      );
    } else {
      canvas.drawLine(
        Offset(thumbCenter.dx, thumbRect.top + 4),
        Offset(thumbCenter.dx, thumbRect.bottom - 4),
        Paint()
          ..color = colors.grip
          ..strokeWidth = 1
          ..strokeCap = StrokeCap.round,
      );
    }
    canvas.restore();
  }

  void _paintMarkers(Canvas canvas, Size size, bool vertical) {
    for (final tick in _faderTicks) {
      final emphasize = centerNotch && tick.pos == 50;
      final len = _tickLength[emphasize ? _TickSize.major : tick.size]!;
      final tone = emphasize ? _tickEmphasize : _tickTone;
      final paint = Paint()..color = tone;

      if (vertical) {
        final y = tick.pos / 100 * size.height;
        final cx = size.width / 2;
        // Left tick (extends left from gap).
        canvas.drawRect(
          Rect.fromLTWH(
            cx - _tickGap - len,
            y - _tickThickness / 2,
            len,
            _tickThickness,
          ),
          paint,
        );
        // Right tick.
        canvas.drawRect(
          Rect.fromLTWH(
            cx + _tickGap,
            y - _tickThickness / 2,
            len,
            _tickThickness,
          ),
          paint,
        );
      } else {
        final x = tick.pos / 100 * size.width;
        final cy = size.height / 2;
        canvas.drawRect(
          Rect.fromLTWH(
            x - _tickThickness / 2,
            cy - _tickGap - len,
            _tickThickness,
            len,
          ),
          paint,
        );
        canvas.drawRect(
          Rect.fromLTWH(
            x - _tickThickness / 2,
            cy + _tickGap,
            _tickThickness,
            len,
          ),
          paint,
        );
      }
    }

    if (centerNotch) {
      final paint = Paint()..color = _tickTone;
      if (vertical) {
        canvas.drawRRect(
          RRect.fromRectAndRadius(
            Rect.fromCenter(
              center: Offset(size.width / 2, size.height / 2),
              width: 14,
              height: 2,
            ),
            const Radius.circular(2),
          ),
          paint,
        );
      } else {
        canvas.drawRRect(
          RRect.fromRectAndRadius(
            Rect.fromCenter(
              center: Offset(size.width / 2, size.height / 2),
              width: 2,
              height: 14,
            ),
            const Radius.circular(2),
          ),
          paint,
        );
      }
    }
  }

  @override
  bool shouldRepaint(covariant _FaderPainter oldDelegate) {
    return t != oldDelegate.t ||
        orientation != oldDelegate.orientation ||
        colors != oldDelegate.colors ||
        showIndicator != oldDelegate.showIndicator ||
        showMarkers != oldDelegate.showMarkers ||
        centerNotch != oldDelegate.centerNotch ||
        crossfaderTrack != oldDelegate.crossfaderTrack ||
        dragging != oldDelegate.dragging;
  }
}
