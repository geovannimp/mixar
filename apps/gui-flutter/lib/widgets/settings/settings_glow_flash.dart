import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// Primary border flash that holds briefly then fades out (~1s).
///
/// Animates only a border overlay; [child] is built once and not rebuilt
/// on each animation tick.
class SettingsGlowFlash extends StatefulWidget {
  const new({required this.child, this.active = false, super.key});

  final Widget child;
  final bool active;

  @override
  State<SettingsGlowFlash> createState() => _SettingsGlowFlashState();
}

class _SettingsGlowFlashState extends State<SettingsGlowFlash>
    with SingleTickerProviderStateMixin {
  late final AnimationController _controller;
  late final CurvedAnimation _curve;
  late final Animation<double> _glow;

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 1000),
    );
    // Hold full glow ~20%, then ease out.
    _curve = CurvedAnimation(
      parent: _controller,
      curve: const Interval(0.2, 1, curve: Curves.easeOut),
    );
    _glow = Tween<double>(begin: 1, end: 0).animate(_curve);
    if (widget.active) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted && widget.active) {
          _controller.forward(from: 0);
        }
      });
    }
  }

  @override
  void didUpdateWidget(SettingsGlowFlash oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.active && !oldWidget.active) {
      _controller.forward(from: 0);
    } else if (!widget.active && oldWidget.active) {
      _controller.stop();
      _controller.reset();
    }
  }

  @override
  void dispose() {
    _curve.dispose();
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    // Always keep AnimatedBuilder so true→false / completed flash does not
    // remount [child] (overlay collapses when t <= 0).
    final theme = context.theme;
    final radius = theme.style.borderRadius.lg;
    final borderW = theme.style.borderWidth;
    final borderBase = theme.colors.border;
    final primary = theme.colors.primary;
    return AnimatedBuilder(
      animation: _glow,
      child: widget.child,
      builder: (context, child) {
        // Controller at 0 maps Tween(begin:1) → 1; only paint while active.
        final t = widget.active ? _glow.value : 0.0;
        if (t <= 0) {
          return child!;
        }
        final color = Color.lerp(borderBase, primary, t)!;
        return Stack(
          children: [
            child!,
            Positioned.fill(
              child: IgnorePointer(
                child: DecoratedBox(
                  decoration: BoxDecoration(
                    borderRadius: radius,
                    border: Border.all(color: color, width: borderW),
                  ),
                ),
              ),
            ),
          ],
        );
      },
    );
  }
}
