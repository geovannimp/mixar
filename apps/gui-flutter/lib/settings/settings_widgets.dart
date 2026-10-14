import 'package:flutter/widgets.dart';
import 'package:gui_flutter/settings/settings_field.dart';
import 'package:gui_flutter/shell/m_card.dart';
import 'package:gui_flutter/shell/mixar_select.dart';
import 'package:gui_flutter/shell/mixar_switch.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';

class SettingsToggle extends StatelessWidget {
  const new({
    required this.label,
    required this.value,
    required this.onChanged,
    super.key,
    this.labelStyle,
  });

  final String label;
  final bool value;
  final ValueChanged<bool> onChanged;
  final TextStyle? labelStyle;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return Row(
      children: [
        Expanded(
          child: Text(
            label,
            style:
                labelStyle ??
                theme.typography.body.sm.copyWith(
                  color: theme.colors.foreground,
                  fontWeight: FontWeight.w600,
                ),
          ),
        ),
        SizedBox(
          height: 23,
          child: FittedBox(
            child: MixarSwitch(value: value, onChanged: onChanged),
          ),
        ),
      ],
    );
  }
}

/// Titled settings card: required title + subtitle above optional [child].
///
/// Pass [trailing] for header actions (e.g. Update All, enable toggles). Omit
/// [child] when the card is header-only (toggle off, no fields to show).
class SettingsSectionCard extends StatelessWidget {
  const new({
    required this.title,
    required this.subtitle,
    super.key,
    this.trailing,
    this.child,
  });

  final String title;
  final String subtitle;
  final Widget? trailing;
  final Widget? child;

  @override
  Widget build(BuildContext context) {
    final header = SettingsSectionHeader(title: title, description: subtitle);
    return MCard(
      child: Padding(
        padding: const .fromLTRB(16, 16, 16, 16),
        child: Column(
          crossAxisAlignment: .start,
          spacing: 16,
          children: [
            if (trailing == null)
              header
            else
              Row(
                crossAxisAlignment: .start,
                spacing: 12,
                children: [
                  Expanded(child: header),
                  trailing!,
                ],
              ),
            if (child != null) child!,
          ],
        ),
      ),
    );
  }
}

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
    // Inactive + never started: bare child. Otherwise keep AnimatedBuilder so
    // the child element is not remounted when the overlay collapses (t <= 0).
    if (!widget.active && _controller.isDismissed) {
      return widget.child;
    }
    final theme = context.theme;
    final radius = theme.style.borderRadius.lg;
    final borderW = theme.style.borderWidth;
    final borderBase = theme.colors.border;
    final primary = theme.colors.primary;
    return AnimatedBuilder(
      animation: _glow,
      child: widget.child,
      builder: (context, child) {
        final t = _glow.value;
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

class SettingsSelect<T> extends StatelessWidget {
  const new({
    required this.value,
    required this.options,
    required this.labelBuilder,
    required this.onChanged,
    super.key,
    this.subtitleBuilder,
    this.enabled = true,
    this.dialogTitle,
  });

  final T value;
  final List<T> options;
  final String Function(T value) labelBuilder;
  final String Function(T value)? subtitleBuilder;
  final ValueChanged<T> onChanged;
  final bool enabled;

  /// Title for the mobile dialog picker (usually the [SettingsField] label).
  final String? dialogTitle;

  @override
  Widget build(BuildContext context) {
    return MixarSelect<T>(
      value: value,
      options: options,
      labelBuilder: labelBuilder,
      subtitleBuilder: subtitleBuilder,
      onChanged: onChanged,
      enabled: enabled,
      dialogTitle: dialogTitle,
    );
  }
}
