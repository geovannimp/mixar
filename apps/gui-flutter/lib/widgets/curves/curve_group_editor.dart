import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/models/curve_editor.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/src/rust/api/event_mirrors.dart';
import 'package:gui_flutter/widgets/curves/curve_chart.dart';
import 'package:gui_flutter/widgets/settings/settings_field.dart';
import 'package:gui_flutter/widgets/settings/settings_numeric_stepper.dart';
import 'package:gui_flutter/widgets/settings/settings_select.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// Point, segment, and channel editor for one custom curve group.
class CurveGroupEditor extends StatefulWidget {
  const new({required this.group, required this.onChanged, super.key});

  final CurveGroupSpec group;
  final ValueChanged<CurveGroupSpec> onChanged;

  @override
  State<CurveGroupEditor> createState() => _CurveGroupEditorState();
}

class _CurveGroupEditorState extends State<CurveGroupEditor> {
  var _channel = 0;
  var _selectedPoint = 0;

  CurveSpecSpec get _curve => widget.group.curves[_channel];

  @override
  void didUpdateWidget(covariant CurveGroupEditor oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.group.id != widget.group.id) {
      _channel = 0;
      _selectedPoint = 0;
      return;
    }
    _channel = _channel.clamp(0, widget.group.curves.length - 1);
    _selectedPoint = _selectedPoint.clamp(0, _curve.points.length - 1);
  }

  void _replaceCurve(CurveSpecSpec curve) {
    final curves = [...widget.group.curves];
    curves[_channel] = curve;
    widget.onChanged(copyCurveGroup(widget.group, curves: curves));
  }

  void _replaceSegment(int index, CurveSegmentSpec segment) {
    final segments = [..._curve.segments];
    segments[index] = segment;
    _replaceCurve(copyCurveSpec(_curve, segments: segments));
  }

  void _addPoint(CurvePointSpec point) {
    final next = insertCurvePoint(_curve, point);
    if (identical(next, _curve)) {
      return;
    }
    _replaceCurve(next);
    final index = next.points.indexWhere((candidate) => candidate.x == point.x);
    if (index >= 0) {
      setState(() => _selectedPoint = index);
    }
  }

  void _addCenteredPoint() {
    final oldXs = _curve.points.map((point) => point.x).toSet();
    final next = addCurvePoint(_curve);
    if (identical(next, _curve)) {
      return;
    }
    final addedIndex = next.points.indexWhere(
      (point) => !oldXs.contains(point.x),
    );
    _replaceCurve(next);
    if (addedIndex >= 0) {
      setState(() => _selectedPoint = addedIndex);
    }
  }

  void _removePoint() {
    final next = removeCurvePoint(_curve, _selectedPoint);
    if (identical(next, _curve)) {
      return;
    }
    _replaceCurve(next);
    setState(() {
      _selectedPoint = (_selectedPoint - 1).clamp(0, next.points.length - 1);
    });
  }

  void _movePoint(int index, double x, double y) {
    final next = moveCurvePoint(_curve, index, x, y);
    _replaceCurve(next);
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final l10n = AppLocalizations.of(context)!;
    final curve = _curve;
    final selected = _selectedPoint.clamp(0, curve.points.length - 1);
    final point = curve.points[selected];
    final segmentIndex = curveSegmentIndexForPoint(
      selected,
      curve.points.length,
    );
    final segment = segmentIndex == null ? null : curve.segments[segmentIndex];
    final error = curveValidationError(curve);
    final canRemove = selected > 0 && selected < curve.points.length - 1;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      spacing: 14,
      children: [
        if (widget.group.curves.length > 1)
          Row(
            children: [
              for (
                var channel = 0;
                channel < widget.group.curves.length;
                channel++
              )
                Padding(
                  padding: const EdgeInsets.only(right: 8),
                  child: AppButton(
                    size: .sm,
                    mainAxisSize: .min,
                    variant: .outline,
                    selected: channel == _channel,
                    onPress: () => setState(() {
                      _channel = channel;
                      _selectedPoint = 0;
                    }),
                    child: Text(
                      channel == 0 ? l10n.curvesChannelA : l10n.curvesChannelB,
                    ),
                  ),
                ),
            ],
          )
        else
          Text(
            l10n.curvesScalarCurve,
            style: theme.typography.body.xs.copyWith(
              color: theme.colors.mutedForeground,
              fontWeight: FontWeight.w700,
              letterSpacing: 0.8,
            ),
          ),
        CurveChart(
          series: [
            CurveSeries(
              values: sampleCurve(curve),
              color: theme.colors.primary,
              filled: true,
            ),
          ],
          points: curve.points,
          selectedPoint: selected,
          onSelectPoint: (index) => setState(() => _selectedPoint = index),
          onMovePoint: _movePoint,
          onAddPoint: _addPoint,
          semanticLabel: l10n.curvesChartSemantics(widget.group.displayName),
        ),
        if (error != null)
          Text(
            error,
            style: theme.typography.body.xs.copyWith(
              color: theme.colors.destructive,
            ),
          )
        else
          Text(
            l10n.curvesChartHint,
            style: theme.typography.body.xs.copyWith(
              color: theme.colors.mutedForeground,
            ),
          ),
        Wrap(
          spacing: 18,
          runSpacing: 12,
          crossAxisAlignment: WrapCrossAlignment.end,
          children: [
            SettingsField(
              label: l10n.curvesPointX,
              child: SettingsNumericStepper(
                value: point.x,
                min: 0,
                max: 1,
                step: 0.01,
                format: (value) => value.toStringAsFixed(2),
                fieldLabel: l10n.curvesPointX,
                onChanged: (value) => _movePoint(selected, value, point.y),
              ),
            ),
            SettingsField(
              label: l10n.curvesPointY,
              child: SettingsNumericStepper(
                value: point.y,
                min: 0,
                max: 1,
                step: 0.01,
                format: (value) => value.toStringAsFixed(2),
                fieldLabel: l10n.curvesPointY,
                onChanged: (value) => _movePoint(selected, point.x, value),
              ),
            ),
            AppButton(
              size: .sm,
              mainAxisSize: .min,
              variant: .outline,
              onPress: _addCenteredPoint,
              child: Text(l10n.curvesAddPoint),
            ),
            AppButton(
              size: .sm,
              mainAxisSize: .min,
              variant: .outline,
              onPress: canRemove ? _removePoint : null,
              child: Text(l10n.curvesRemovePoint),
            ),
          ],
        ),
        if (segment != null && segmentIndex != null)
          Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            spacing: 10,
            children: [
              SettingsField(
                label: l10n.curvesSegment(segmentIndex + 1),
                child: SettingsSelect<CurveSegmentKind>(
                  dialogTitle: l10n.curvesSegment(segmentIndex + 1),
                  value: segment.kind,
                  options: CurveSegmentKind.values,
                  labelBuilder: (kind) => switch (kind) {
                    CurveSegmentKind.linear => l10n.curvesSegmentLinear,
                    CurveSegmentKind.exponential =>
                      l10n.curvesSegmentExponential,
                    CurveSegmentKind.logarithmic =>
                      l10n.curvesSegmentLogarithmic,
                    CurveSegmentKind.cubicBezier =>
                      l10n.curvesSegmentCubicBezier,
                  },
                  onChanged: (kind) =>
                      _replaceSegment(segmentIndex, curveSegment(kind)),
                ),
              ),
              if (segment.kind == CurveSegmentKind.exponential ||
                  segment.kind == CurveSegmentKind.logarithmic)
                SettingsField(
                  label: l10n.curvesRate,
                  child: SettingsNumericStepper(
                    value: segment.rate,
                    min: -kMaxCurveSegmentRate,
                    max: kMaxCurveSegmentRate,
                    step: 0.5,
                    format: (value) => value.toStringAsFixed(2),
                    fieldLabel: l10n.curvesRate,
                    onChanged: (value) => _replaceSegment(
                      segmentIndex,
                      copyCurveSegment(segment, rate: value),
                    ),
                  ),
                ),
              if (segment.kind == CurveSegmentKind.cubicBezier)
                Wrap(
                  spacing: 18,
                  runSpacing: 12,
                  children: [
                    for (final handle in [
                      (l10n.curvesBezierC1X, segment.c1X, 0),
                      (l10n.curvesBezierC1Y, segment.c1Y, 1),
                      (l10n.curvesBezierC2X, segment.c2X, 2),
                      (l10n.curvesBezierC2Y, segment.c2Y, 3),
                    ])
                      SettingsField(
                        label: handle.$1,
                        child: SettingsNumericStepper(
                          value: handle.$2,
                          min: 0,
                          max: 1,
                          step: 0.05,
                          format: (value) => value.toStringAsFixed(2),
                          fieldLabel: handle.$1,
                          onChanged: (value) =>
                              _replaceSegment(segmentIndex, switch (handle.$3) {
                                0 => copyCurveSegment(segment, c1X: value),
                                1 => copyCurveSegment(segment, c1Y: value),
                                2 => copyCurveSegment(segment, c2X: value),
                                _ => copyCurveSegment(segment, c2Y: value),
                              }),
                        ),
                      ),
                  ],
                ),
            ],
          ),
      ],
    );
  }
}
