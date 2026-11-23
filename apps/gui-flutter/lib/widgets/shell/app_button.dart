import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/theme/mixar_button_style.dart';
import 'package:gui_flutter/widgets/shell/m_tappable.dart' show MTappable;
import 'package:gui_flutter/widgets/shell/mixar_button_body.dart';

/// Settings / library / shell / dialog button (Forui visual parity, [MTappable]).
class AppButton extends StatelessWidget {
  const new({
    required this.child,
    this.onPress,
    this.onSecondaryPress,
    this.variant = MixarButtonVariant.primary,
    this.size = MixarButtonSize.md,
    this.selected = false,
    this.mainAxisSize = MainAxisSize.max,
    this.semanticsLabel,
    this.padding,
    this.backgroundColor,
    this.borderRadius,
    super.key,
  }) : _icon = false;

  const new icon({
    required this.child,
    this.onPress,
    this.onSecondaryPress,
    this.variant = MixarButtonVariant.primary,
    this.size = MixarButtonSize.md,
    this.selected = false,
    this.semanticsLabel,
    this.padding,
    this.backgroundColor,
    super.key,
  }) : _icon = true,
       borderRadius = null,
       mainAxisSize = MainAxisSize.min;

  final Widget child;
  final VoidCallback? onPress;
  final VoidCallback? onSecondaryPress;
  final MixarButtonVariant variant;
  final MixarButtonSize size;
  final bool selected;
  final MainAxisSize mainAxisSize;
  final String? semanticsLabel;
  final EdgeInsetsGeometry? padding;
  final Color? backgroundColor;

  /// Overrides the shared pill radius. The shell titlebar passes the theme's
  /// square step so its chrome stays flat. Icon buttons are always circular
  /// and do not expose this parameter.
  final BorderRadius? borderRadius;

  final bool _icon;

  @override
  Widget build(BuildContext context) {
    return MixarButtonBody(
      density: MixarButtonDensity.app,
      variant: variant,
      size: size,
      selected: selected,
      mainAxisSize: mainAxisSize,
      semanticsLabel: semanticsLabel,
      onPress: onPress,
      onSecondaryPress: onSecondaryPress,
      padding: padding,
      backgroundColor: backgroundColor,
      borderRadius: borderRadius,
      icon: _icon,
      child: child,
    );
  }
}
