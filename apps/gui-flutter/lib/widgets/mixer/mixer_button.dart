import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/theme/mixar_button_style.dart';
import 'package:gui_flutter/widgets/shell/m_tappable.dart' show MTappable;
import 'package:gui_flutter/widgets/shell/mixar_button_body.dart';

/// Dense deck / mixer button (Forui visual parity, [MTappable]).
class MixerButton extends StatelessWidget {
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
  final bool _icon;

  @override
  Widget build(BuildContext context) {
    return MixarButtonBody(
      density: MixarButtonDensity.mixer,
      variant: variant,
      size: size,
      selected: selected,
      mainAxisSize: mainAxisSize,
      semanticsLabel: semanticsLabel,
      onPress: onPress,
      onSecondaryPress: onSecondaryPress,
      padding: padding,
      backgroundColor: backgroundColor,
      icon: _icon,
      child: child,
    );
  }
}
