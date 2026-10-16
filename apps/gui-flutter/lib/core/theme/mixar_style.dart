import 'package:flutter/foundation.dart';
import 'package:flutter/painting.dart';

/// Border radius scale — square chrome; [pill] only for tiny status chips.
@immutable
class MixarBorderRadius {
  const new({
    this.xs2 = BorderRadius.zero,
    this.xs = BorderRadius.zero,
    this.sm = BorderRadius.zero,
    this.md = BorderRadius.zero,
    this.lg = BorderRadius.zero,
    this.xl = BorderRadius.zero,
    this.xl2 = BorderRadius.zero,
    this.xl3 = BorderRadius.zero,
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
