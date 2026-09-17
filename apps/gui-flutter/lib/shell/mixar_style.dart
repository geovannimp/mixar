import 'package:flutter/foundation.dart';
import 'package:flutter/painting.dart';

/// Border radius scale (Forui desktop defaults).
@immutable
class MixarBorderRadius {
  const new({
    this.xs2 = const BorderRadius.all(Radius.circular(4)),
    this.xs = const BorderRadius.all(Radius.circular(6)),
    this.sm = const BorderRadius.all(Radius.circular(8)),
    this.md = const BorderRadius.all(Radius.circular(10)),
    this.lg = const BorderRadius.all(Radius.circular(14)),
    this.xl = const BorderRadius.all(Radius.circular(18)),
    this.xl2 = const BorderRadius.all(Radius.circular(22)),
    this.xl3 = const BorderRadius.all(Radius.circular(26)),
    this.pill = const BorderRadius.all(Radius.circular(100)),
  });

  final BorderRadius xs2;
  final BorderRadius xs;
  final BorderRadius sm;
  final BorderRadius md;
  final BorderRadius lg;
  final BorderRadius xl;
  final BorderRadius xl2;
  final BorderRadius xl3;
  final BorderRadius pill;
}

/// Chrome tokens that aren't colors or type.
@immutable
class MixarStyle {
  const new({
    this.borderRadius = const MixarBorderRadius(),
    this.borderWidth = 1,
  });

  final MixarBorderRadius borderRadius;
  final double borderWidth;
}
