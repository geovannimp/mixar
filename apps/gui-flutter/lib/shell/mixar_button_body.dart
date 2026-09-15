import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';

import 'package:gui_flutter/shell/m_tappable.dart';
import 'package:gui_flutter/shell/mixar_button_style.dart';

/// Shared paint/layout for [MixerButton] / [AppButton].
class MixarButtonBody extends StatelessWidget {
  const MixarButtonBody({
    required this.density,
    required this.variant,
    required this.size,
    required this.selected,
    required this.mainAxisSize,
    required this.onPress,
    required this.icon,
    required this.child,
    this.onSecondaryPress,
    this.semanticsLabel,
    this.padding,
    this.backgroundColor,
    super.key,
  });

  final MixarButtonDensity density;
  final MixarButtonVariant variant;
  final MixarButtonSize size;
  final bool selected;
  final MainAxisSize mainAxisSize;
  final VoidCallback? onPress;
  final VoidCallback? onSecondaryPress;
  final bool icon;
  final Widget child;
  final String? semanticsLabel;
  final EdgeInsetsGeometry? padding;
  final Color? backgroundColor;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return MTappable(
      onPress: onPress,
      onSecondaryPress: onSecondaryPress,
      selected: selected,
      semanticsLabel: semanticsLabel,
      builder: (context, state) {
        final look = resolveMixarButtonLook(
          theme: theme,
          variant: variant,
          size: size,
          density: density,
          state: state,
          icon: icon,
          padding: padding,
          backgroundColor: backgroundColor,
        );
        Widget content = IconTheme.merge(
          data: IconThemeData(color: look.foreground, size: look.iconSize),
          child: DefaultTextStyle.merge(style: look.textStyle, child: child),
        );
        if (!icon) {
          content = Padding(
            padding: look.contentPadding,
            child: Row(
              mainAxisSize: mainAxisSize,
              mainAxisAlignment: MainAxisAlignment.center,
              children: [content],
            ),
          );
        } else {
          content = Padding(padding: look.iconPadding, child: content);
        }

        Widget box = DecoratedBox(
          decoration: look.decoration,
          child: ConstrainedBox(
            constraints: icon
                ? BoxConstraints(
                    minWidth: look.contentConstraints.minHeight,
                    minHeight: look.contentConstraints.minHeight,
                  )
                : look.contentConstraints,
            child: Center(widthFactor: 1, heightFactor: 1, child: content),
          ),
        );

        if (look.showFocusOutline) {
          box = DecoratedBox(
            decoration: BoxDecoration(
              borderRadius: look.borderRadius,
              border: Border.all(color: look.focusColor, width: 2),
            ),
            child: box,
          );
        }
        return box;
      },
    );
  }
}
