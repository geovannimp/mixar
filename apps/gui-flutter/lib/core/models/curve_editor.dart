import 'dart:math' as math;

import 'package:gui_flutter/src/rust/api/event_mirrors.dart';

/// Smallest number of points a validated curve may hold.
const kMinCurvePoints = 2;

/// Largest number of points a validated curve may hold.
const kMaxCurvePoints = 256;

/// Largest absolute rate accepted by exponential/logarithmic segments.
const kMaxCurveSegmentRate = 20.0;

/// Maximum custom curve groups the library accepts.
const kMaxCustomCurveGroups = 64;

/// Maximum curve sets the library accepts.
const kMaxCurveSets = 32;

/// Maximum members in one curve set.
const kMaxCurveSetMembers = 32;

/// Minimum members a curve set needs to morph (the engine's sweep contract).
const kMinCurveSetMembers = 2;

/// Maximum curves a group may hold.
const kMaxCurveGroupChannels = 16;

/// Built-in crossfader preset ids that custom ids may not shadow.
const kCrossfaderPresetIds = <String>['constant_power', 'linear', 'fast_cut'];

/// Reserved preset names that cannot be selected yet but still own their id.
const kReservedCurvePresetIds = <String>{
  'constant_power',
  'linear',
  'fast_cut',
  'full',
  'intermediate',
  'slow_fade',
  'slow_cut',
  'transition',
  'logarithmic',
};

/// Display name for one of the selectable built-in presets.
String curvePresetName(String id) => switch (id) {
  'constant_power' => 'Constant power',
  'linear' => 'Linear',
  'fast_cut' => 'Fast cut',
  _ => id,
};

/// A two-point straight line from `startY` to `endY`.
CurveSpecSpec linearCurveSpec(double startY, double endY) {
  return CurveSpecSpec(
    points: [
      CurvePointSpec(x: 0, y: startY),
      CurvePointSpec(x: 1, y: endY),
    ],
    segments: [curveSegment(CurveSegmentKind.linear)],
  );
}

/// A fresh group: A/B decks when [channels] is two, a scalar curve otherwise.
CurveGroupSpec newCurveGroup({
  required String id,
  required String displayName,
  required int channels,
}) {
  final curves = channels == 2
      ? [linearCurveSpec(1, 0), linearCurveSpec(0, 1)]
      : [linearCurveSpec(0, 1)];
  return CurveGroupSpec(id: id, displayName: displayName, curves: curves);
}

/// A fresh two-member set, so it satisfies the engine's sweep contract before
/// the user edits it.
CurveSetSpec newCurveSet({required String id, required String displayName}) {
  return CurveSetSpec(
    id: id,
    displayName: displayName,
    members: const [
      CurveRefSpec(id: 'constant_power'),
      CurveRefSpec(id: 'linear'),
    ],
  );
}

/// Segment defaults for [kind]; switching kinds resets unused fields.
CurveSegmentSpec curveSegment(
  CurveSegmentKind kind, {
  double rate = 1,
  double c1x = 0.33,
  double c1y = 0.33,
  double c2x = 0.67,
  double c2y = 0.67,
}) {
  return CurveSegmentSpec(
    kind: kind,
    rate:
        kind == CurveSegmentKind.exponential ||
            kind == CurveSegmentKind.logarithmic
        ? rate
        : 0,
    c1X: kind == CurveSegmentKind.cubicBezier ? c1x : 0,
    c1Y: kind == CurveSegmentKind.cubicBezier ? c1y : 0,
    c2X: kind == CurveSegmentKind.cubicBezier ? c2x : 0,
    c2Y: kind == CurveSegmentKind.cubicBezier ? c2y : 0,
  );
}

/// A snake_case id suggestion derived from a display name.
String slugifyCurveId(String value) {
  var slug = value
      .toLowerCase()
      .replaceAll(RegExp('[^a-z0-9]+'), '_')
      .replaceAll(RegExp(r'^_+|_+$'), '');
  if (slug.isEmpty) {
    slug = 'curve';
  }
  if (!RegExp('^[a-z]').hasMatch(slug)) {
    slug = 'curve_$slug';
  }
  if (slug.length > 64) {
    slug = slug.substring(0, 64).replaceFirst(RegExp(r'_+$'), '');
  }
  return slug;
}

/// Validation message for a group/set id, or `null` when it can be saved.
String? curveIdError(
  String id, {
  Iterable<String> otherGroupIds = const [],
  Iterable<String> otherSetIds = const [],
}) {
  if (id.isEmpty) {
    return 'Curve ids cannot be empty.';
  }
  if (id.length > 64) {
    return 'Curve ids are limited to 64 characters.';
  }
  if (!RegExp(r'^[a-z][a-z0-9_]*$').hasMatch(id)) {
    return 'Use lowercase letters, digits and underscores; '
        'start with a letter.';
  }
  if (kReservedCurvePresetIds.contains(id)) {
    return 'That id is reserved for a built-in curve.';
  }
  if (otherGroupIds.contains(id)) {
    return 'A curve with that id already exists.';
  }
  if (otherSetIds.contains(id)) {
    return 'A curve set with that id already exists.';
  }
  return null;
}

/// Validation message for one curve, or `null` when it is well formed.
String? curveValidationError(CurveSpecSpec curve) {
  final count = curve.points.length;
  if (count < kMinCurvePoints || count > kMaxCurvePoints) {
    return 'A curve needs between $kMinCurvePoints and '
        '$kMaxCurvePoints points, got $count.';
  }
  if (curve.segments.length != count - 1) {
    return 'A curve with $count points needs ${count - 1} segments, got '
        '${curve.segments.length}.';
  }
  for (var index = 0; index < count; index++) {
    final point = curve.points[index];
    if (!point.x.isFinite || !point.y.isFinite) {
      return 'Point ${index + 1} has a non-finite coordinate.';
    }
    if (point.x < 0 || point.x > 1) {
      return 'Point ${index + 1} x must be between 0 and 1.';
    }
    if (point.y < 0 || point.y > 1) {
      return 'Point ${index + 1} y must be between 0 and 1.';
    }
  }
  if (curve.points.first.x != 0) {
    return 'The curve must start at x = 0.';
  }
  if (curve.points.last.x != 1) {
    return 'The curve must end at x = 1.';
  }
  var previous = curve.points.first.x;
  for (var index = 1; index < count; index++) {
    final x = curve.points[index].x;
    if (x <= previous) {
      return 'Point ${index + 1} x must be greater than $previous.';
    }
    previous = x;
  }
  for (var index = 0; index < curve.segments.length; index++) {
    final segment = curve.segments[index];
    switch (segment.kind) {
      case CurveSegmentKind.linear:
        break;
      case CurveSegmentKind.exponential:
      case CurveSegmentKind.logarithmic:
        if (!segment.rate.isFinite ||
            segment.rate.abs() > kMaxCurveSegmentRate) {
          return 'Segment ${index + 1} rate must be finite and at most '
              '$kMaxCurveSegmentRate in absolute value.';
        }
      case CurveSegmentKind.cubicBezier:
        for (final value in [
          segment.c1X,
          segment.c1Y,
          segment.c2X,
          segment.c2Y,
        ]) {
          if (!value.isFinite || value < 0 || value > 1) {
            return 'Segment ${index + 1} Bézier handles must be '
                'between 0 and 1.';
          }
        }
    }
  }
  return null;
}

/// Validation message for one group, or `null` when it is well formed.
String? curveGroupValidationError(
  CurveGroupSpec group, {
  Iterable<String> otherGroupIds = const [],
  Iterable<String> otherSetIds = const [],
}) {
  final idError = curveIdError(
    group.id,
    otherGroupIds: otherGroupIds.where((id) => id != group.id),
    otherSetIds: otherSetIds,
  );
  if (idError != null) {
    return idError;
  }
  if (group.curves.isEmpty || group.curves.length > kMaxCurveGroupChannels) {
    return 'A curve group needs between 1 and $kMaxCurveGroupChannels curves.';
  }
  for (var channel = 0; channel < group.curves.length; channel++) {
    final error = curveValidationError(group.curves[channel]);
    if (error != null) {
      return group.curves.length > 1 ? 'Channel ${channel + 1}: $error' : error;
    }
  }
  return null;
}

/// Whether [reference] resolves to a preset or a group in [library].
bool curveRefExists(CurveRefSpec reference, CurveLibrarySpec library) {
  if (kCrossfaderPresetIds.contains(reference.id)) {
    return true;
  }
  return library.groups.any((group) => group.id == reference.id);
}

/// Validation message for one set, or `null` when it is well formed.
String? curveSetValidationError(CurveSetSpec set, CurveLibrarySpec library) {
  final idError = curveIdError(
    set.id,
    otherGroupIds: library.groups.map((group) => group.id),
    otherSetIds: library.sets
        .where((other) => other.id != set.id)
        .map((other) => other.id),
  );
  if (idError != null) {
    return idError;
  }
  if (set.members.length < kMinCurveSetMembers ||
      set.members.length > kMaxCurveSetMembers) {
    return 'A curve set needs between $kMinCurveSetMembers and '
        '$kMaxCurveSetMembers members.';
  }
  for (final member in set.members) {
    if (!curveRefExists(member, library)) {
      return 'Curve set member "${member.id}" does not exist.';
    }
    final channels = curveRefChannels(member, library);
    if (channels != 2) {
      return 'A crossfader curve set can only hold A/B curves.';
    }
  }
  return null;
}

/// Validation message for a whole library, or `null` when it can be saved.
String? curveLibraryValidationError(CurveLibrarySpec library) {
  if (library.groups.length > kMaxCustomCurveGroups) {
    return 'A curve library holds at most $kMaxCustomCurveGroups curves.';
  }
  if (library.sets.length > kMaxCurveSets) {
    return 'A curve library holds at most $kMaxCurveSets curve sets.';
  }
  final groupIds = library.groups.map((group) => group.id).toList();
  final setIds = library.sets.map((set) => set.id).toList();
  for (var index = 0; index < library.groups.length; index++) {
    final group = library.groups[index];
    final error = curveGroupValidationError(
      group,
      otherGroupIds: groupIds.sublist(0, index),
      otherSetIds: setIds,
    );
    if (error != null) {
      return error;
    }
  }
  for (final set in library.sets) {
    final error = curveSetValidationError(set, library);
    if (error != null) {
      return error;
    }
  }
  return null;
}

/// Number of outputs [reference] resolves to, or `0` when it is missing.
int curveRefChannels(CurveRefSpec reference, CurveLibrarySpec library) {
  if (kCrossfaderPresetIds.contains(reference.id)) {
    return 2;
  }
  for (final group in library.groups) {
    if (group.id == reference.id) {
      return group.curves.length;
    }
  }
  return 0;
}

/// Evaluate one curve at normalized [x], matching `curve-core`'s shapes.
double evaluateCurve(CurveSpecSpec curve, double x) {
  if (curve.points.isEmpty) {
    return 0;
  }
  final first = curve.points.first;
  if (x.isNaN || x <= first.x) {
    return first.y;
  }
  final last = curve.points.last;
  if (x >= last.x) {
    return last.y;
  }
  var index = 0;
  for (var cursor = 1; cursor < curve.points.length; cursor++) {
    if (curve.points[cursor].x > x) {
      break;
    }
    index = cursor;
  }
  index = index.clamp(0, curve.points.length - 2);
  final start = curve.points[index];
  final end = curve.points[index + 1];
  final span = end.x - start.x;
  final t = span > 0 ? (x - start.x) / span : 0.0;
  final segment = index < curve.segments.length
      ? curve.segments[index]
      : curveSegment(CurveSegmentKind.linear);
  return start.y + (end.y - start.y) * curveSegmentShape(segment, t);
}

/// Normalized `0..1` shape of one segment, matching `curve-core`.
double curveSegmentShape(CurveSegmentSpec segment, double t) {
  final clamped = t.isNaN ? 0.0 : t.clamp(0.0, 1.0);
  if (clamped <= 0) {
    return 0;
  }
  if (clamped >= 1) {
    return 1;
  }
  switch (segment.kind) {
    case CurveSegmentKind.linear:
      return clamped;
    case CurveSegmentKind.exponential:
      final rate = segment.rate;
      if (!rate.isFinite || rate.abs() < 1e-4) {
        return clamped;
      }
      final denominator = math.exp(rate) - 1;
      if (denominator == 0 || !denominator.isFinite) {
        return clamped;
      }
      return (math.exp(rate * clamped) - 1) / denominator;
    case CurveSegmentKind.logarithmic:
      final rate = segment.rate;
      if (!rate.isFinite || rate.abs() < 1e-4) {
        return clamped;
      }
      final factor = math.exp(rate) - 1;
      if (!factor.isFinite) {
        return clamped;
      }
      final inner = 1 + factor * clamped;
      if (inner <= 0) {
        return clamped;
      }
      return (math.log(inner) / rate).clamp(0.0, 1.0);
    case CurveSegmentKind.cubicBezier:
      var low = 0.0;
      var high = 1.0;
      var u = clamped;
      for (var step = 0; step < 32; step++) {
        final x = _bezierAxis(u, segment.c1X, segment.c2X);
        if ((x - clamped).abs() < 1e-6) {
          break;
        }
        if (x < clamped) {
          low = u;
        } else {
          high = u;
        }
        u = 0.5 * (low + high);
      }
      return _bezierAxis(u, segment.c1Y, segment.c2Y);
  }
}

double _bezierAxis(double u, double p1, double p2) {
  final oneMinus = 1 - u;
  return 3 * oneMinus * oneMinus * u * p1 +
      3 * oneMinus * u * u * p2 +
      u * u * u;
}

/// Evaluate a built-in preset as its two A/B channels.
List<double> evaluateCurvePreset(String id, double? parameter, double x) {
  final clamped = x.clamp(0.0, 1.0);
  return switch (id) {
    'constant_power' => [
      math.cos(clamped * math.pi / 2),
      math.sin(clamped * math.pi / 2),
    ],
    'linear' => [1 - clamped, clamped],
    'fast_cut' => _fastCutValues(parameter ?? 0.15, clamped),
    _ => [0, 0],
  };
}

List<double> _fastCutValues(double cut, double x) {
  final fade = cut <= 0 || cut > 1 ? 0.15 : cut;
  return [((1 - x) / fade).clamp(0.0, 1.0), (x / fade).clamp(0.0, 1.0)];
}

/// Evaluate every channel of [group] at [x].
List<double> evaluateCurveGroup(CurveGroupSpec group, double x) {
  return [for (final curve in group.curves) evaluateCurve(curve, x)];
}

/// Evaluate a curve set at morph position [morph] (`0..1`).
///
/// Adjacent members are linearly blended exactly like `curve-core`'s
/// `ResolvedCurveGroup`, so the morph preview matches the engine.
List<double> evaluateCurveSet(
  CurveSetSpec set,
  CurveLibrarySpec library,
  double morph,
  double x,
) {
  final members = [
    for (final reference in set.members) _memberValues(reference, library, x),
  ];
  if (members.isEmpty) {
    return [0, 0];
  }
  final channels = members.first.length;
  final normalized = morph.isNaN ? 0.0 : morph.clamp(0.0, 1.0);
  final last = members.length - 1;
  final scaled = normalized * last;
  final lower = scaled.floor().clamp(0, last);
  final upper = math.min(lower + 1, last);
  final fraction = (scaled - lower).clamp(0.0, 1.0);
  final result = [...members[lower]];
  if (upper != lower && fraction > 0) {
    final next = members[upper];
    for (
      var channel = 0;
      channel < math.min(channels, next.length);
      channel++
    ) {
      result[channel] += (next[channel] - result[channel]) * fraction;
    }
  }
  return result;
}

List<double> _memberValues(
  CurveRefSpec reference,
  CurveLibrarySpec library,
  double x,
) {
  if (kCrossfaderPresetIds.contains(reference.id)) {
    return evaluateCurvePreset(reference.id, reference.parameter, x);
  }
  for (final group in library.groups) {
    if (group.id == reference.id) {
      return evaluateCurveGroup(group, x);
    }
  }
  return const [0, 0];
}

/// Display label for a set member.
String curveSetMemberLabel(CurveRefSpec reference, CurveLibrarySpec library) {
  if (kCrossfaderPresetIds.contains(reference.id)) {
    final parameter = reference.parameter;
    final name = curvePresetName(reference.id);
    return parameter == null ? name : '$name (${parameter.toStringAsFixed(2)})';
  }
  for (final group in library.groups) {
    if (group.id == reference.id) {
      return group.displayName.isEmpty ? group.id : group.displayName;
    }
  }
  return reference.id;
}

/// Copy a point with selected fields replaced.
CurvePointSpec copyCurvePoint(CurvePointSpec point, {double? x, double? y}) {
  return CurvePointSpec(x: x ?? point.x, y: y ?? point.y);
}

/// Copy a segment with selected fields replaced.
CurveSegmentSpec copyCurveSegment(
  CurveSegmentSpec segment, {
  CurveSegmentKind? kind,
  double? rate,
  double? c1X,
  double? c1Y,
  double? c2X,
  double? c2Y,
}) {
  return CurveSegmentSpec(
    kind: kind ?? segment.kind,
    rate: rate ?? segment.rate,
    c1X: c1X ?? segment.c1X,
    c1Y: c1Y ?? segment.c1Y,
    c2X: c2X ?? segment.c2X,
    c2Y: c2Y ?? segment.c2Y,
  );
}

/// Copy a curve with selected fields replaced.
CurveSpecSpec copyCurveSpec(
  CurveSpecSpec curve, {
  List<CurvePointSpec>? points,
  List<CurveSegmentSpec>? segments,
}) {
  return CurveSpecSpec(
    points: points ?? curve.points,
    segments: segments ?? curve.segments,
  );
}

/// Copy a group with selected fields replaced.
CurveGroupSpec copyCurveGroup(
  CurveGroupSpec group, {
  String? id,
  String? displayName,
  List<CurveSpecSpec>? curves,
}) {
  return CurveGroupSpec(
    id: id ?? group.id,
    displayName: displayName ?? group.displayName,
    curves: curves ?? group.curves,
  );
}

/// Copy a set with selected fields replaced.
CurveSetSpec copyCurveSet(
  CurveSetSpec set, {
  String? id,
  String? displayName,
  List<CurveRefSpec>? members,
}) {
  return CurveSetSpec(
    id: id ?? set.id,
    displayName: displayName ?? set.displayName,
    members: members ?? set.members,
  );
}

/// Copy a library with selected fields replaced.
CurveLibrarySpec copyCurveLibrary(
  CurveLibrarySpec library, {
  List<CurveGroupSpec>? groups,
  List<CurveSetSpec>? sets,
}) {
  return CurveLibrarySpec(
    groups: groups ?? library.groups,
    sets: sets ?? library.sets,
  );
}

/// Move one point, keeping endpoints fixed and internal x strictly ordered.
CurveSpecSpec moveCurvePoint(
  CurveSpecSpec curve,
  int index,
  double x,
  double y,
) {
  if (index < 0 || index >= curve.points.length) {
    return curve;
  }
  const epsilon = 0.0005;
  final last = curve.points.length - 1;
  final lower = index == 0
      ? 0.0
      : math.max(0.0, curve.points[index - 1].x + epsilon);
  final upper = index == last
      ? 1.0
      : math.min(1.0, curve.points[index + 1].x - epsilon);
  if (lower > upper) {
    return curve;
  }
  final nextX = index == 0
      ? 0.0
      : (index == last ? 1.0 : x.clamp(lower, upper));
  final nextY = y.clamp(0.0, 1.0);
  final points = [...curve.points];
  points[index] = CurvePointSpec(x: nextX, y: nextY);
  return copyCurveSpec(curve, points: points);
}

/// Insert [point] between the two points whose span contains its x.
CurveSpecSpec insertCurvePoint(CurveSpecSpec curve, CurvePointSpec point) {
  const epsilon = 0.0005;
  if (curve.points.length >= kMaxCurvePoints ||
      point.x <= epsilon ||
      point.x >= 1 - epsilon ||
      curve.points.any((existing) => (existing.x - point.x).abs() < epsilon)) {
    return curve;
  }
  final insertAt = curve.points.indexWhere((existing) => existing.x > point.x);
  if (insertAt <= 0 || insertAt >= curve.points.length) {
    return curve;
  }
  final points = [...curve.points]..insert(insertAt, point);
  final segments = [...curve.segments];
  final inherited = insertAt - 1 < segments.length
      ? segments[insertAt - 1]
      : curveSegment(CurveSegmentKind.linear);
  segments.insert(insertAt, copyCurveSegment(inherited));
  return copyCurveSpec(curve, points: points, segments: segments);
}

/// Insert a point in the widest span, at the curve's current y there.
CurveSpecSpec addCurvePoint(CurveSpecSpec curve) {
  if (curve.points.length >= kMaxCurvePoints) {
    return curve;
  }
  var widest = 0;
  var gap = curve.points[1].x - curve.points[0].x;
  for (var index = 1; index < curve.points.length - 1; index++) {
    final next = curve.points[index + 1].x - curve.points[index].x;
    if (next > gap) {
      gap = next;
      widest = index;
    }
  }
  final x = (curve.points[widest].x + curve.points[widest + 1].x) / 2;
  final y = evaluateCurve(curve, x);
  return insertCurvePoint(curve, CurvePointSpec(x: x, y: y));
}

/// Remove an internal point, keeping the incoming segment for the merged span.
CurveSpecSpec removeCurvePoint(CurveSpecSpec curve, int index) {
  if (index <= 0 ||
      index >= curve.points.length - 1 ||
      curve.points.length <= kMinCurvePoints) {
    return curve;
  }
  final points = [...curve.points]..removeAt(index);
  final segments = [...curve.segments];
  if (index < segments.length) {
    segments.removeAt(index);
  }
  return copyCurveSpec(curve, points: points, segments: segments);
}

/// The segment affected by [pointIndex]: outgoing, or incoming for the last
/// point. `null` when the curve has no segments.
int? curveSegmentIndexForPoint(int pointIndex, int pointCount) {
  final segmentCount = pointCount - 1;
  if (segmentCount <= 0) {
    return null;
  }
  if (pointIndex < segmentCount) {
    return pointIndex;
  }
  return math.max(0, segmentCount - 1);
}
