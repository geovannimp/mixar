import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/theme/deck_accent.dart';
import 'package:gui_flutter/core/theme/mixar_palette.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// Soft snap near mid — narrow so centering is optional (Tauri `CENTER_SNAP_THRESHOLD`).
const kFaderCenterSnapThreshold = 0.8;

enum FaderOrientation { vertical, horizontal }

enum _TickSize { major, minor }

class _FaderTick {
  const new(this.pos, this.size);
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

const Map<_TickSize, double> _tickLength = {
  _TickSize.major: 10.0,
  _TickSize.minor: 6.0,
};

/// Deck A/B/C/D and neutral fader tokens.
@immutable
class _FaderColors {
  const new({required this.indicator, required this.grip});

  final Color indicator;
  final Color grip;

  // Value equality keeps the painter's `colors != old` check honest when a
  // caller passes a fresh instance on rebuild.
  @override
  bool operator ==(Object other) =>
      other is _FaderColors &&
      other.indicator == indicator &&
      other.grip == grip;

  @override
  int get hashCode => Object.hash(indicator, grip);

  // Palette steps, not literal hex: the tone's 400/600 rung is the grip (the
  // same step the deck's labels, badges and knob arcs pick), and the lane
  // carries a quiet tint behind it — 900 on dark chrome, 200 on warm bone — so
  // a full-height fader reads as a soft bar instead of a neon stripe. No alpha:
  // solid steps composite the same over slot and card.
  static _FaderColors forAccent(DeckAccent accent, Brightness brightness) {
    // Neutral faders are chrome, not deck identity: the indicator stays mid
    // grey (zinc-500) and the grip is author-tuned separately — zinc-200 reads
    // close to the light plate, so it relies on the lane frame for shape.
    if (accent == DeckAccent.neutral) {
      return const _FaderColors(
        indicator: MixarZinc.s500,
        grip: MixarZinc.s200,
      );
    }
    final tone = deckTone(accent);
    return _FaderColors(
      indicator: brightness == Brightness.dark ? tone.s900 : tone.s300,
      grip: brightness == Brightness.dark ? tone.s500 : tone.s300,
    );
  }
}

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
///
/// The grip is a plate riding a 6px groove, wider than the lane so the deck
/// colour and the cap read at a glance (a 10px cap disappeared into the slot).
/// Cross-axis sizes are the plate width; the along-axis extent sets the travel.
///
/// The horizontal plate is 20 tall on purpose: every horizontal fader sits in a
/// 24px row (crossfader strip, master/cue in `master_strip.dart`), so it keeps
/// 2px clear at each edge instead of filling the row.
const kFaderThumbV = Size(26, 12);
const kFaderThumbH = Size(12, 20);

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
  var next = snapFaderToStep(raw, step, origin: min).clamp(min, max);
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
  new({
    required this.value,
    required this.onValueChange,
    this.min = 0,
    this.max = 100,
    this.step = 1,
    this.orientation = FaderOrientation.vertical,
    this.accent = DeckAccent.neutral,
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
  final DeckAccent accent;
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
  const new(this.delta);
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
        (widget.value + delta).clamp(widget.min, widget.max),
        min: widget.min,
        max: widget.max,
        step: widget.step,
        centerNotch: widget.centerNotch,
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final colors = _FaderColors.forAccent(
      widget.accent,
      theme.colors.brightness,
    );
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
                        grooveColor: theme.colors.background,
                        edgeColor: theme.colors.border,
                        faceColor: theme.colors.brightness == Brightness.dark
                            ? MixarZinc.s600
                            : MixarZinc.s400,
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
  new({
    required this.t,
    required this.orientation,
    required this.colors,
    required this.grooveColor,
    required this.edgeColor,
    required this.faceColor,
    required this.showIndicator,
    required this.showMarkers,
    required this.centerNotch,
    required this.crossfaderTrack,
    required this.dragging,
  });

  final double t;
  final FaderOrientation orientation;
  final _FaderColors colors;

  /// Recessed lane the grip rides in (`theme.colors.background`, the window
  /// tone: a slot cut through the card the mixer sits on).
  final Color grooveColor;

  /// Hairline around the groove and the grip (`theme.colors.border`).
  final Color edgeColor;

  /// Grip plate.
  final Color faceColor;

  final bool showIndicator;
  final bool showMarkers;
  final bool centerNotch;
  final bool crossfaderTrack;
  final bool dragging;

  static const _grooveThickness = 6.0;
  static const _tickGap = 6.0;
  static const _tickThickness = 2.0;
  static const _tickTone = Color(0x3371717a); // zinc-500 @ 20%
  static const _tickEmphasize = Color(0x4071717a); // zinc-500 @ 25%
  static const _crossStart = Color(0x140ea5e9); // sky-500 @ 8%
  static const _crossMid = Color(0x1a71717a); // zinc-500 @ 10%
  static const _crossEnd = Color(0x14f43f5e); // rose-500 @ 8%

  @override
  void paint(Canvas canvas, Size size) {
    final vertical = orientation == FaderOrientation.vertical;
    final half = faderThumbExtentAlongAxis(orientation) / 2;
    final travel = faderTravelLength(size, orientation);
    // The lane ends exactly under the grip at each end stop: thumb travel plus
    // one grip extent is the whole box, so a value at min/max never leaves a
    // stub of empty slot sticking out past the cap.
    final trackRect = vertical
        ? Rect.fromCenter(
            center: Offset(size.width / 2, size.height / 2),
            width: _grooveThickness,
            height: size.height,
          )
        : Rect.fromCenter(
            center: Offset(size.width / 2, size.height / 2),
            width: size.width,
            height: _grooveThickness,
          );

    // Square slot: Mixar chrome has no rounded rects, only pills.
    final trackRRect = RRect.fromRectAndRadius(trackRect, Radius.zero);
    // The lane is a slot cut into the panel, not a painted line: the 1px
    // [edgeColor] frame with the recessed surface inside it. The frame is what
    // draws the lane at all in light mode, where the window tone (FAFAFA) and
    // the card (FFFFFF) are five levels apart; in dark mode it is the same
    // hairline the panels use, so the slot reads as chrome either way. Filled
    // bands rather than a stroke, so the frame is exactly the outer 1px.
    final laneRect = vertical
        ? Rect.fromCenter(
            center: trackRect.center,
            width: _grooveThickness - 2,
            height: trackRect.height - 2,
          )
        : Rect.fromCenter(
            center: trackRect.center,
            width: trackRect.width - 2,
            height: _grooveThickness - 2,
          );
    canvas.drawRRect(trackRRect, Paint()..color = edgeColor);
    canvas.drawRRect(
      RRect.fromRectAndRadius(laneRect, Radius.zero),
      Paint()..color = grooveColor,
    );

    // Value fill and the crossfader tint sit inside the frame.
    if (crossfaderTrack && !vertical) {
      final paint = Paint()
        ..shader = const LinearGradient(
          colors: [_crossStart, _crossMid, _crossEnd],
        ).createShader(laneRect);
      canvas.drawRRect(RRect.fromRectAndRadius(laneRect, Radius.zero), paint);
    }

    if (showMarkers) {
      _paintMarkers(canvas, size, vertical, half, travel);
    }

    if (showIndicator) {
      final thumbCenter = faderThumbCenter(
        size: size,
        orientation: orientation,
        t: t,
      );
      final indicator = vertical
          ? Rect.fromLTRB(
              laneRect.left,
              thumbCenter.dy,
              laneRect.right,
              laneRect.bottom,
            )
          : Rect.fromLTRB(
              laneRect.left,
              laneRect.top,
              thumbCenter.dx,
              laneRect.bottom,
            );
      canvas.drawRRect(
        RRect.fromRectAndRadius(indicator, Radius.zero),
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

    final thumbRRect = RRect.fromRectAndRadius(thumbRect, Radius.zero);
    canvas.drawRRect(thumbRRect, Paint()..color = faceColor);

    // Deck-coloured grip line across the plate — the same mark the EQ knobs
    // carry as a radial tick.
    if (vertical) {
      canvas.drawLine(
        Offset(thumbRect.left, thumbCenter.dy),
        Offset(thumbRect.right, thumbCenter.dy),
        Paint()
          ..color = colors.grip
          ..strokeWidth = 2,
      );
    } else {
      canvas.drawLine(
        Offset(thumbCenter.dx, thumbRect.top),
        Offset(thumbCenter.dx, thumbRect.bottom),
        Paint()
          ..color = colors.grip
          ..strokeWidth = 2,
      );
    }
    canvas.restore();
  }

  void _paintMarkers(
    Canvas canvas,
    Size size,
    bool vertical,
    double half,
    double travel,
  ) {
    for (final tick in _faderTicks) {
      final emphasize = centerNotch && tick.pos == 50;
      final len = _tickLength[emphasize ? _TickSize.major : tick.size]!;
      final tone = emphasize ? _tickEmphasize : _tickTone;
      final paint = Paint()..color = tone;
      // Tick 0 = max end (top/left of travel), 100 = min end — matches thumb.
      final axis = half + (tick.pos / 100) * travel;

      if (vertical) {
        final y = axis;
        final cx = size.width / 2;
        canvas.drawRect(
          Rect.fromLTWH(
            cx - _tickGap - len,
            y - _tickThickness / 2,
            len,
            _tickThickness,
          ),
          paint,
        );
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
        final x = axis;
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
      final mid = faderThumbCenter(
        size: size,
        orientation: orientation,
        t: 0.5,
      );
      if (vertical) {
        canvas.drawRRect(
          RRect.fromRectAndRadius(
            Rect.fromCenter(center: mid, width: 14, height: 2),
            Radius.zero,
          ),
          paint,
        );
      } else {
        canvas.drawRRect(
          RRect.fromRectAndRadius(
            Rect.fromCenter(center: mid, width: 2, height: 14),
            Radius.zero,
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
        grooveColor != oldDelegate.grooveColor ||
        edgeColor != oldDelegate.edgeColor ||
        faceColor != oldDelegate.faceColor ||
        showIndicator != oldDelegate.showIndicator ||
        showMarkers != oldDelegate.showMarkers ||
        centerNotch != oldDelegate.centerNotch ||
        crossfaderTrack != oldDelegate.crossfaderTrack ||
        dragging != oldDelegate.dragging;
  }
}
