import 'package:flutter/widgets.dart';

/// The platform's own scroll physics.
///
/// `SilkyScroll` defaults to a bare [ScrollPhysics], which has no boundary
/// conditions. Passing this through keeps the app's existing edge behaviour
/// (clamping on desktop/Android, bouncing on iOS/macOS) while SilkyScroll adds
/// the animated wheel/trackpad interpolation on top.
ScrollPhysics mixarScrollPhysics(BuildContext context) =>
    ScrollConfiguration.of(context).getScrollPhysics(context);

/// SilkyScroll's decay rate for the app's glides.
///
/// SilkyScroll 2.6.4 derives its exponential smoothing rate from
/// `decayLogFactor / silkyScrollDuration`; the default is `12 / 1.6s ≈ 7.5/s`.
/// Doubling it settles the glide in about half the time, which keeps the
/// motion closer to a constant-speed scroll instead of a long ease-out tail.
const double kMixarSilkyDecayLogFactor = 24;
