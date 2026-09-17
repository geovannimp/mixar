import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Indeterminate loader sizes (Forui circular-progress body font Size parity).
enum MLoaderSize { xs, sm, md, lg, xl }

/// Spinning Lucide [LucideIcons.loaderCircle] indeterminate loader.
class MLoader extends StatefulWidget {
  const MLoader({
    this.size = MLoaderSize.md,
    this.color,
    this.semanticsLabel,
    super.key,
  });

  final MLoaderSize size;
  final Color? color;
  final String? semanticsLabel;

  @override
  State<MLoader> createState() => _MLoaderState();
}

class _MLoaderState extends State<MLoader> with SingleTickerProviderStateMixin {
  late final AnimationController _controller;

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(
      vsync: this,
      duration: const Duration(seconds: 1),
    )..repeat();
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  double _iconSize(FThemeData theme) {
    final body = theme.typography.body;
    return switch (widget.size) {
      MLoaderSize.xs => body.xs.fontSize ?? 12,
      MLoaderSize.sm => body.sm.fontSize ?? 14,
      MLoaderSize.md => body.md.fontSize ?? 16,
      MLoaderSize.lg => body.lg.fontSize ?? 18,
      MLoaderSize.xl => body.xl.fontSize ?? 20,
    };
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final color =
        widget.color ??
        IconTheme.of(context).color ??
        DefaultTextStyle.of(context).style.color;
    return Semantics(
      label: widget.semanticsLabel ?? 'Loading',
      child: RotationTransition(
        turns: _controller,
        child: Icon(
          LucideIcons.loaderCircle,
          size: _iconSize(theme),
          color: color,
        ),
      ),
    );
  }
}
