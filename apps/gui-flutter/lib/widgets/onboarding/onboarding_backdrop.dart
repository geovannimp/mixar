import 'dart:math' as math;

import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// Whether [context] asks for motion-free UI (OS "reduce motion").
bool onboardingMotionDisabled(BuildContext context) =>
    MediaQuery.maybeOf(context)?.disableAnimations ?? false;

/// Full-window atmosphere behind the setup steps: an accent glow, a faint dot
/// grid, and a spectral band along the bottom edge.
///
/// Deliberately outside the app's card language — setup is the one screen that
/// is not the mixer, so it gets a stage instead of panels.
class OnboardingBackdrop extends StatefulWidget {
  const new({this.animate = true, super.key});

  final bool animate;

  @override
  State<OnboardingBackdrop> createState() => _OnboardingBackdropState();
}

class _OnboardingBackdropState extends State<OnboardingBackdrop>
    with SingleTickerProviderStateMixin {
  late final AnimationController _controller = AnimationController(
    vsync: this,
    duration: const Duration(seconds: 12),
  );

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    // Reduce-motion can resolve after the first build (MediaQuery), so settle
    // the loop here rather than only in initState.
    _syncAnimation();
  }

  @override
  void didUpdateWidget(OnboardingBackdrop oldWidget) {
    super.didUpdateWidget(oldWidget);
    // `animate` can flip while the inherited dependencies stay put.
    _syncAnimation();
  }

  void _syncAnimation() {
    final animate = widget.animate && !onboardingMotionDisabled(context);
    if (animate && !_controller.isAnimating) {
      _controller.repeat();
    } else if (!animate && _controller.isAnimating) {
      _controller.stop();
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return Stack(
      fit: StackFit.expand,
      children: [
        // Canvas, bloom and grid never change while the page is open: their own
        // boundary keeps ~1.8k dot rects from being re-issued every frame.
        RepaintBoundary(
          child: CustomPaint(painter: _StillPainter(theme: theme)),
        ),
        _AnimatedPaint(
          controller: _controller,
          painter: (phase) => _SpectrumPainter(theme: theme, phase: phase),
        ),
      ],
    );
  }
}

/// A painter driven by [controller], isolated in its own repaint boundary.
///
/// Both animated layers of the stage use it: the wrapper is what keeps a
/// fractional tick from repainting the still canvas behind it.
class _AnimatedPaint extends StatelessWidget {
  const new({required this.controller, required this.painter});

  final Animation<double> controller;

  /// Built from the controller's current value on every tick.
  final CustomPainter Function(double phase) painter;

  @override
  Widget build(BuildContext context) {
    return RepaintBoundary(
      child: AnimatedBuilder(
        animation: controller,
        // No size: the caller's tight constraints stretch it to the stage.
        builder: (_, __) => CustomPaint(painter: painter(controller.value)),
      ),
    );
  }
}

/// Height of the footer strip the backdrop leaves clear.
const double kOnboardingFooterHeight = 88;

/// Canvas, accent bloom and dot grid. Static: no animation input.
class _StillPainter extends CustomPainter {
  const new({required this.theme});

  final MixarThemeData theme;

  static const _dotStep = 34.0;

  @override
  void paint(Canvas canvas, Size size) {
    final colors = theme.colors;
    final accent = colors.accent;
    final dark = colors.brightness == Brightness.dark;

    canvas.drawRect(Offset.zero & size, Paint()..color = colors.background);

    // Accent bloom behind the headline column: the only warm light on the
    // stage, so the eye lands on the copy first.
    for (final (center, radius, alpha) in [
      (
        Offset(size.width * 0.16, size.height * 0.42),
        math.max(size.width, size.height) * 0.55,
        dark ? 0.30 : 0.20,
      ),
      (
        Offset(size.width * 0.78, size.height * 0.34),
        math.max(size.width, size.height) * 0.42,
        dark ? 0.18 : 0.12,
      ),
    ]) {
      canvas.drawCircle(
        center,
        radius,
        Paint()
          ..shader = RadialGradient(
            colors: [
              accent.withValues(alpha: alpha),
              accent.withValues(alpha: 0),
            ],
            stops: const [0, 0.75],
          ).createShader(Rect.fromCircle(center: center, radius: radius)),
      );
    }

    // Dot grid: a studio floor, barely there. One path keeps it a single call.
    final dots = Path();
    for (var x = _dotStep / 2; x < size.width; x += _dotStep) {
      for (var y = _dotStep / 2; y < size.height; y += _dotStep) {
        dots.addRect(Rect.fromLTWH(x, y, 1.4, 1.4));
      }
    }
    canvas.drawPath(
      dots,
      Paint()..color = colors.foreground.withValues(alpha: 0.05),
    );
  }

  @override
  bool shouldRepaint(covariant _StillPainter old) => old.theme != theme;
}

/// Spectral silhouette along the bottom of the canvas — the mixer's own visual,
/// reduced to one soft hill so it never competes with the cards.
class _SpectrumPainter extends CustomPainter {
  const new({required this.theme, required this.phase});

  final MixarThemeData theme;
  final double phase;

  @override
  void paint(Canvas canvas, Size size) {
    final accent = theme.colors.accent;
    // Above the footer strip, so the actions never sit on moving pixels.
    final baseline = size.height - kOnboardingFooterHeight;
    if (baseline <= 0) {
      return;
    }
    final height = math.min(130, baseline * 0.3).toDouble();
    final top = baseline - height;

    final fill = Path()..moveTo(0, baseline);
    final crest = Path();
    const steps = 64;
    for (var i = 0; i <= steps; i++) {
      final t = i / steps;
      final low = math.sin(t * math.pi * 2.4 + phase * math.pi * 2);
      final high = math.sin(t * math.pi * 5.7 - phase * math.pi * 4 + 0.8);
      final envelope = math.pow(math.sin(t * math.pi), 0.7).toDouble();
      final y =
          baseline -
          (0.35 + 0.42 * (0.6 * low + 0.4 * high)) * envelope * height;
      final point = Offset(size.width * t, y.clamp(top, baseline));
      fill.lineTo(point.dx, point.dy);
      if (i == 0) {
        crest.moveTo(point.dx, point.dy);
      } else {
        crest.lineTo(point.dx, point.dy);
      }
    }
    fill
      ..lineTo(size.width, baseline)
      ..close();

    // The fill fades into the canvas; the crest reads as a playhead.
    canvas
      ..drawPath(
        fill,
        Paint()
          ..shader = LinearGradient(
            begin: Alignment.topCenter,
            end: Alignment.bottomCenter,
            colors: [
              accent.withValues(alpha: 0.22),
              accent.withValues(alpha: 0.02),
            ],
          ).createShader(Rect.fromLTWH(0, top, size.width, height)),
      )
      ..drawPath(
        crest,
        Paint()
          ..style = PaintingStyle.stroke
          ..strokeWidth = 1
          ..color = accent.withValues(alpha: 0.45),
      );
  }

  @override
  bool shouldRepaint(covariant _SpectrumPainter old) =>
      old.phase != phase || old.theme != theme;
}
