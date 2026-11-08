import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/models/curve_editor.dart';
import 'package:gui_flutter/src/rust/api/event_mirrors.dart';

CurveSpecSpec _curve(
  List<CurvePointSpec> points, [
  List<CurveSegmentSpec>? segments,
]) {
  return CurveSpecSpec(
    points: points,
    segments:
        segments ??
        [
          for (var index = 0; index < points.length - 1; index++)
            curveSegment(CurveSegmentKind.linear),
        ],
  );
}

CurveGroupSpec _group(String id, [int channels = 2]) {
  return newCurveGroup(id: id, displayName: id, channels: channels);
}

void main() {
  group('curve ids', () {
    test('slugify turns a display name into a stable id', () {
      expect(slugifyCurveId('My Tight Curve'), 'my_tight_curve');
      expect(slugifyCurveId('A/B Blends'), 'a_b_blends');
      expect(slugifyCurveId('30% cut'), 'curve_30_cut');
      expect(slugifyCurveId('   '), 'curve');
    });

    test('rejects reserved, malformed, and duplicate ids', () {
      expect(curveIdError(''), isNotNull);
      expect(curveIdError('Bad-Id'), isNotNull);
      expect(curveIdError('linear'), contains('reserved'));
      expect(curveIdError('mine', otherGroupIds: ['mine']), isNotNull);
      expect(curveIdError('mine', otherSetIds: ['mine']), isNotNull);
      expect(curveIdError('mine'), isNull);
    });
  });

  group('curve validation', () {
    test('accepts a straight line and evaluates its endpoints', () {
      final curve = linearCurveSpec(1, 0);
      expect(curveValidationError(curve), isNull);
      expect(evaluateCurve(curve, 0), 1);
      expect(evaluateCurve(curve, 0.25), 0.75);
      expect(evaluateCurve(curve, 1), 0);
    });

    test('rejects missing endpoints and non-increasing x', () {
      expect(
        curveValidationError(
          _curve([CurvePointSpec(x: 0.1, y: 0), CurvePointSpec(x: 1, y: 1)]),
        ),
        contains('start at x = 0'),
      );
      expect(
        curveValidationError(
          _curve([
            CurvePointSpec(x: 0, y: 0),
            CurvePointSpec(x: 0.5, y: 0.5),
            CurvePointSpec(x: 0.5, y: 0.6),
            CurvePointSpec(x: 1, y: 1),
          ]),
        ),
        contains('must be greater'),
      );
    });

    test('rejects out-of-range values and segment parameters', () {
      expect(
        curveValidationError(
          _curve([CurvePointSpec(x: 0, y: -0.1), CurvePointSpec(x: 1, y: 1)]),
        ),
        contains('between 0 and 1'),
      );
      expect(
        curveValidationError(
          _curve(
            [CurvePointSpec(x: 0, y: 0), CurvePointSpec(x: 1, y: 1)],
            [
              CurveSegmentSpec(
                kind: CurveSegmentKind.exponential,
                rate: 21,
                c1X: 0,
                c1Y: 0,
                c2X: 0,
                c2Y: 0,
              ),
            ],
          ),
        ),
        contains('rate'),
      );
      expect(
        curveValidationError(
          _curve(
            [CurvePointSpec(x: 0, y: 0), CurvePointSpec(x: 1, y: 1)],
            [
              CurveSegmentSpec(
                kind: CurveSegmentKind.cubicBezier,
                rate: 0,
                c1X: 0.2,
                c1Y: 1.4,
                c2X: 0.8,
                c2Y: 0.2,
              ),
            ],
          ),
        ),
        contains('Bézier'),
      );
    });

    test('segment shapes hit exact endpoints and bend as configured', () {
      final exponential = curveSegment(CurveSegmentKind.exponential, rate: 4);
      expect(curveSegmentShape(exponential, 0), 0);
      expect(curveSegmentShape(exponential, 1), 1);
      expect(curveSegmentShape(exponential, 0.5), lessThan(0.5));

      final logarithmic = curveSegment(CurveSegmentKind.logarithmic, rate: 4);
      expect(curveSegmentShape(logarithmic, 0), 0);
      expect(curveSegmentShape(logarithmic, 1), 1);
      expect(curveSegmentShape(logarithmic, 0.5), greaterThan(0.5));

      final bezier = curveSegment(CurveSegmentKind.cubicBezier);
      expect(curveSegmentShape(bezier, 0), 0);
      expect(curveSegmentShape(bezier, 1), 1);
    });

    test('preset values match the engine laws', () {
      final smooth = evaluateCurvePreset('constant_power', null, 0);
      expect(smooth[0], closeTo(1, 1e-6));
      expect(smooth[1], closeTo(0, 1e-6));
      final linear = evaluateCurvePreset('linear', null, 0.25);
      expect(linear[0], closeTo(0.75, 1e-6));
      expect(linear[1], closeTo(0.25, 1e-6));
      final cut = evaluateCurvePreset('fast_cut', 0.1, 0.05);
      expect(cut[0], 1);
      expect(cut[1], closeTo(0.5, 1e-6));
    });
  });

  group('point editing', () {
    test('moves internal points without crossing neighbours', () {
      var curve = _curve([
        CurvePointSpec(x: 0, y: 0),
        CurvePointSpec(x: 0.5, y: 0.5),
        CurvePointSpec(x: 1, y: 1),
      ]);
      curve = moveCurvePoint(curve, 1, 0.9, 0.1);
      expect(curve.points[1].x, 0.9);
      expect(curve.points[1].y, 0.1);
      curve = moveCurvePoint(curve, 1, -0.5, 2);
      expect(curve.points[1].x, closeTo(0.0005, 1e-9));
      expect(curve.points[1].y, 1);
      curve = moveCurvePoint(curve, 0, 0.4, 0);
      expect(curve.points.first.x, 0, reason: 'endpoints are locked to x=0');
      curve = moveCurvePoint(curve, 2, 0.4, 1);
      expect(curve.points.last.x, 1, reason: 'endpoints are locked to x=1');
    });

    test('adds a point in the widest span and removes it again', () {
      final base = linearCurveSpec(0, 1);
      final added = addCurvePoint(base);
      expect(added.points.length, 3);
      expect(added.points[1].x, closeTo(0.5, 1e-6));
      expect(added.segments.length, 2);
      expect(added.segments.first.kind, CurveSegmentKind.linear);

      final removed = removeCurvePoint(added, 1);
      expect(removed.points.length, 2);
      expect(removed.segments.length, 1);
      expect(
        removeCurvePoint(base, 0).points.length,
        2,
        reason: 'endpoints cannot be removed',
      );
    });

    test('insert preserves the segment kind on both new spans', () {
      final curve = _curve(
        [CurvePointSpec(x: 0, y: 0), CurvePointSpec(x: 1, y: 1)],
        [curveSegment(CurveSegmentKind.exponential, rate: 2)],
      );
      final inserted = insertCurvePoint(curve, CurvePointSpec(x: 0.4, y: 0.2));
      expect(inserted.points.length, 3);
      expect(inserted.segments.length, 2);
      expect(inserted.segments[0].kind, CurveSegmentKind.exponential);
      expect(inserted.segments[1].kind, CurveSegmentKind.exponential);
      expect(inserted.segments[0].rate, 2);
    });
  });

  group('curve sets', () {
    test('morph blends adjacent members like the engine', () {
      final first = _group('first');
      final second = newCurveGroup(
        id: 'second',
        displayName: 'second',
        channels: 2,
      );
      final inverted = copyCurveGroup(
        second,
        curves: [linearCurveSpec(0, 1), linearCurveSpec(1, 0)],
      );
      final library = CurveLibrarySpec(
        groups: [first, inverted],
        sets: [
          CurveSetSpec(
            id: 'sweep',
            displayName: 'Sweep',
            members: [
              CurveRefSpec(id: 'first'),
              CurveRefSpec(id: 'second'),
            ],
          ),
        ],
      );
      final values = evaluateCurveSet(library.sets.first, library, 0.5, 0.25);
      expect(values[0], closeTo(0.5, 1e-6));
      expect(values[1], closeTo(0.5, 1e-6));
      expect(curveLibraryValidationError(library), isNull);
    });

    test('rejects unknown members, short sets, and scalar members', () {
      final scalar = _group('scalar', 1);
      final library = CurveLibrarySpec(
        groups: [scalar],
        sets: [
          CurveSetSpec(
            id: 'broken',
            displayName: 'Broken',
            members: [
              CurveRefSpec(id: 'missing'),
              CurveRefSpec(id: 'linear'),
            ],
          ),
        ],
      );
      expect(curveLibraryValidationError(library), contains('does not exist'));

      final short = CurveLibrarySpec(
        groups: [],
        sets: [
          CurveSetSpec(
            id: 'short',
            displayName: 'Short',
            members: [CurveRefSpec(id: 'linear')],
          ),
        ],
      );
      expect(curveLibraryValidationError(short), contains('between 2 and'));

      final scalarMember = CurveLibrarySpec(
        groups: [scalar],
        sets: [
          CurveSetSpec(
            id: 'scalar_set',
            displayName: 'Scalar set',
            members: [
              CurveRefSpec(id: 'linear'),
              CurveRefSpec(id: 'scalar'),
            ],
          ),
        ],
      );
      expect(curveLibraryValidationError(scalarMember), contains('A/B curves'));
    });
  });
}
