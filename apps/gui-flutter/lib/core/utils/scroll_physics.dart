import 'package:flutter/widgets.dart';

/// The platform's own scroll physics.
///
/// `SilkyScroll` defaults to a bare [ScrollPhysics], which has no boundary
/// conditions. Passing this through keeps the app's existing edge behaviour
/// (clamping on desktop/Android, bouncing on iOS/macOS) while SilkyScroll adds
/// the animated wheel/trackpad interpolation on top.
ScrollPhysics mixarScrollPhysics(BuildContext context) =>
    ScrollConfiguration.of(context).getScrollPhysics(context);
