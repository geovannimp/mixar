import 'dart:math' as math;

import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/models/curve_editor.dart';
import 'package:gui_flutter/src/rust/api/event_mirrors.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// One sampled line drawn by [CurveChart].
class CurveSeries {
  const new({
    required this.values,
    required this.color,
    this.strokeWidth = 2,
    this.filled = false,
  });

  /// Sampled `0..1` values, evenly spaced from x = 0 to x = 1.
  final List<double> values;
  final Color color;
  final double strokeWidth;

  /// Fill the area under the line with a faint wash.
  final bool filled;
}

/// Editable or read-only normalized curve chart.
///
/// Pass [points] with the edit callbacks to get draggable handles. Tapping the
/// plot adds a point through [onAddPoint]; tapping a handle selects it.
class CurveChart extends StatefulWidget {
  const new({
    required this.series,
    super.key,
    this.points,
    this.selectedPoint,
    this.onSelectPoint,
    this.onMovePoint,
    this.onAddPoint,
    this.height = 240,
    this.semanticLabel,
  });

  final List<CurveSeries> series;

  /// Editable handles. `null` renders a preview without handles.
  final List<CurvePointSpec>? points;
  final int? selectedPoint;
  final ValueChanged<int>? onSelectPoint;
  final void Function(int index, double x, double y)? onMovePoint;
  final ValueChanged<CurvePointSpec>? onAddPoint;
  final double height;
  final String? semanticLabel;

  @override
  State<CurveChart> createState() => _CurveChartState();
}

class _CurveChartState extends State<CurveChart> {
  static const _padding = EdgeInsets.fromLTRB(36, 14, 14, 24);
  static const _hitRadius = 18.0;
  int? _dragging;

  Rect _plotRect(Size size) => Rect.fromLTRB(
    _padding.left,
    _padding.top,
    size.width - _padding.right,
    size.height - _padding.bottom,
  );

  Offset _toPixel(CurvePointSpec point, Rect plot) => Offset(
    plot.left + point.x.clamp(0.0, 1.0) * plot.width,
    plot.bottom - point.y.clamp(0.0, 1.0) * plot.height,
  );

  CurvePointSpec _toPoint(Offset pixel, Rect plot) => CurvePointSpec(
    x: ((pixel.dx - plot.left) / plot.width).clamp(0.0, 1.0),
    y: ((plot.bottom - pixel.dy) / plot.height).clamp(0.0, 1.0),
  );

  int? _hitTest(Offset local, Rect plot) {
    final points = widget.points;
    if (points == null) {
      return null;
    }
    var best = _hitRadius;
    int? index;
    for (var cursor = 0; cursor < points.length; cursor++) {
      final distance = (local - _toPixel(points[cursor], plot)).distance;
      if (distance <= best) {
        best = distance;
        index = cursor;
      }
    }
    return index;
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return LayoutBuilder(
      builder: (context, constraints) {
        final size = Size(constraints.maxWidth, widget.height);
        final plot = _plotRect(size);
        return Semantics(
          label: widget.semanticLabel,
          child: GestureDetector(
            behavior: HitTestBehavior.opaque,
            onTapUp: (details) {
              final hit = _hitTest(details.localPosition, plot);
              if (hit != null) {
                widget.onSelectPoint?.call(hit);
                return;
              }
              final add = widget.onAddPoint;
              if (add != null) {
                add(_toPoint(details.localPosition, plot));
              }
            },
            onPanStart: (details) {
              final hit = _hitTest(details.localPosition, plot);
              if (hit == null) {
                return;
              }
              setState(() => _dragging = hit);
              widget.onSelectPoint?.call(hit);
            },
            onPanUpdate: (details) {
              final index = _dragging;
              final move = widget.onMovePoint;
              if (index == null || move == null) {
                return;
              }
              final point = _toPoint(details.localPosition, plot);
              move(index, point.x, point.y);
            },
            onPanEnd: (_) => setState(() => _dragging = null),
            onPanCancel: () => setState(() => _dragging = null),
            child: CustomPaint(
              size: size,
              painter: _CurvePainter(
                series: widget.series,
                points: widget.points,
                selectedPoint: widget.selectedPoint,
                plot: plot,
                grid: theme.colors.border,
                background: theme.colors.background,
                handleFill: theme.colors.card,
                handleStroke: theme.colors.primary,
                label: theme.colors.mutedForeground,
                textStyle: theme.typography.body.xs,
              ),
            ),
          ),
        );
      },
    );
  }
}

class _CurvePainter extends CustomPainter {
  const new({
    required this.series,
    required this.points,
    required this.selectedPoint,
    required this.plot,
    required this.grid,
    required this.background,
    required this.handleFill,
    required this.handleStroke,
    required this.label,
    required this.textStyle,
  });

  final List<CurveSeries> series;
  final List<CurvePointSpec>? points;
  final int? selectedPoint;
  final Rect plot;
  final Color grid;
  final Color background;
  final Color handleFill;
  final Color handleStroke;
  final Color label;
  final TextStyle textStyle;

  @override
  void paint(Canvas canvas, Size size) {
    canvas
      ..drawRect(
        Offset.zero & size,
        Paint()
          ..color = background
          ..style = PaintingStyle.fill,
      )
      ..drawRect(
        plot,
        Paint()
          ..color = grid
          ..style = PaintingStyle.stroke
          ..strokeWidth = 1,
      );

    final gridPaint = Paint()
      ..color = grid.withValues(alpha: 0.55)
      ..strokeWidth = 1;
    for (var step = 1; step < 4; step++) {
      final fraction = step / 4;
      final x = plot.left + plot.width * fraction;
      final y = plot.bottom - plot.height * fraction;
      canvas
        ..drawLine(Offset(x, plot.top), Offset(x, plot.bottom), gridPaint)
        ..drawLine(Offset(plot.left, y), Offset(plot.right, y), gridPaint);
    }

    _labelText(canvas, '0', Offset(plot.left - 22, plot.bottom - 7));
    _labelText(canvas, '1', Offset(plot.left - 22, plot.top - 7));
    _labelText(canvas, '0', Offset(plot.left - 3, plot.bottom + 4));
    _labelText(canvas, '1', Offset(plot.right - 6, plot.bottom + 4));

    for (final line in series) {
      _paintSeries(canvas, line);
    }
    _paintHandles(canvas);
  }

  void _labelText(Canvas canvas, String text, Offset offset) {
    TextPainter(
        text: TextSpan(
          text: text,
          style: textStyle.copyWith(color: label),
        ),
        textDirection: TextDirection.ltr,
      )
      ..layout()
      ..paint(canvas, offset);
  }

  void _paintSeries(Canvas canvas, CurveSeries line) {
    final values = line.values;
    if (values.isEmpty) {
      return;
    }
    final path = Path();
    for (var index = 0; index < values.length; index++) {
      final x =
          plot.left +
          (values.length == 1 ? 0 : index / (values.length - 1)) * plot.width;
      final y = plot.bottom - values[index].clamp(0.0, 1.0) * plot.height;
      if (index == 0) {
        path.moveTo(x, y);
      } else {
        path.lineTo(x, y);
      }
    }
    if (line.filled) {
      final fill = Path.from(path)
        ..lineTo(plot.right, plot.bottom)
        ..lineTo(plot.left, plot.bottom)
        ..close();
      canvas.drawPath(
        fill,
        Paint()
          ..color = line.color.withValues(alpha: 0.14)
          ..style = PaintingStyle.fill,
      );
    }
    canvas.drawPath(
      path,
      Paint()
        ..color = line.color
        ..style = PaintingStyle.stroke
        ..strokeWidth = line.strokeWidth
        ..strokeCap = StrokeCap.round
        ..strokeJoin = StrokeJoin.round,
    );
  }

  void _paintHandles(Canvas canvas) {
    final handles = points;
    if (handles == null) {
      return;
    }
    for (var index = 0; index < handles.length; index++) {
      final selected = index == selectedPoint;
      final center = Offset(
        plot.left + handles[index].x.clamp(0.0, 1.0) * plot.width,
        plot.bottom - handles[index].y.clamp(0.0, 1.0) * plot.height,
      );
      final radius = selected ? 6.5 : 4.5;
      canvas
        ..drawCircle(
          center,
          radius,
          Paint()
            ..color = selected ? handleStroke : handleFill
            ..style = PaintingStyle.fill,
        )
        ..drawCircle(
          center,
          radius,
          Paint()
            ..color = handleStroke
            ..style = PaintingStyle.stroke
            ..strokeWidth = selected ? 2.5 : 1.5,
        );
    }
  }

  @override
  bool shouldRepaint(covariant _CurvePainter oldDelegate) {
    return oldDelegate.series != series ||
        oldDelegate.points != points ||
        oldDelegate.selectedPoint != selectedPoint ||
        oldDelegate.plot != plot ||
        oldDelegate.grid != grid ||
        oldDelegate.background != background;
  }
}

/// Sample [curve] into the evenly spaced values a [CurveSeries] wants.
List<double> sampleCurve(CurveSpecSpec curve, {int samples = 96}) {
  final count = math.max(1, samples);
  return [
    for (var index = 0; index < count; index++)
      evaluateCurve(curve, index / (count - 1)),
  ];
}

/// Sample every channel of [group] for a preview.
List<CurveSeries> groupPreviewSeries(
  CurveGroupSpec group,
  Color first,
  Color second, {
  int samples = 96,
}) {
  final count = math.max(1, samples);
  return [
    for (var channel = 0; channel < group.curves.length; channel++)
      CurveSeries(
        values: [
          for (var index = 0; index < count; index++)
            evaluateCurve(group.curves[channel], index / (count - 1)),
        ],
        color: channel == 0 ? first : second,
        strokeWidth: 1.6,
        filled: channel == 0,
      ),
  ];
}
