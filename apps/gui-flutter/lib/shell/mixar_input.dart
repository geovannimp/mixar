import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:shadcn_ui/shadcn_ui.dart';

/// Mixar text field over [ShadInput].
class MixarInput extends StatelessWidget {
  const new({
    super.key,
    this.controller,
    this.initialValue,
    this.focusNode,
    this.onChanged,
    this.onSubmitted,
    this.hint,
    this.label,
    this.leading,
    this.trailing,
    this.enabled = true,
    this.readOnly = false,
    this.textAlign = TextAlign.start,
    this.keyboardType,
    this.textInputAction,
    this.inputFormatters,
    this.style,
    this.padding,
    this.borderless = false,
    this.fillColor,
    this.maxLines = 1,
  });

  final TextEditingController? controller;
  final String? initialValue;
  final FocusNode? focusNode;
  final ValueChanged<String>? onChanged;
  final ValueChanged<String>? onSubmitted;
  final String? hint;
  final Widget? label;
  final Widget? leading;
  final Widget? trailing;
  final bool enabled;
  final bool readOnly;
  final TextAlign textAlign;
  final TextInputType? keyboardType;
  final TextInputAction? textInputAction;
  final List<TextInputFormatter>? inputFormatters;
  final TextStyle? style;

  /// Overrides [ShadInput]'s theme padding. Used by dense toolbars that need a
  /// shorter field than the default vertical padding produces.
  final EdgeInsetsGeometry? padding;

  /// Drops [ShadInput]'s rounded border, for a flat field that fills a bar.
  final bool borderless;

  /// Paints the field's background. Implies [borderless] when set.
  final Color? fillColor;
  final int? maxLines;

  /// Flat chrome: no border in any state, optional fill.
  ///
  /// `canMerge: false` replaces the theme's bordered decoration rather than
  /// merging with it.
  static ShadDecoration _flatDecoration(Color? color) => ShadDecoration(
    canMerge: false,
    color: color,
    border: ShadBorder.none,
    focusedBorder: ShadBorder.none,
    errorBorder: ShadBorder.none,
    secondaryBorder: ShadBorder.none,
    secondaryFocusedBorder: ShadBorder.none,
    secondaryErrorBorder: ShadBorder.none,
  );

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final flat = borderless || fillColor != null;
    final field = ShadInput(
      controller: controller,
      initialValue: controller == null ? initialValue : null,
      focusNode: focusNode,
      onChanged: onChanged,
      onSubmitted: onSubmitted,
      enabled: enabled,
      readOnly: readOnly,
      textAlign: textAlign,
      keyboardType: keyboardType,
      textInputAction: textInputAction,
      inputFormatters: inputFormatters,
      style: style,
      maxLines: maxLines,
      padding: padding,
      decoration: flat ? _flatDecoration(fillColor) : null,
      placeholder: hint == null
          ? null
          : Text(
              hint!,
              style: theme.typography.body.sm.copyWith(
                color: theme.colors.mutedForeground,
              ),
            ),
      leading: leading,
      trailing: trailing,
    );
    if (label == null) {
      return field;
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      mainAxisSize: MainAxisSize.min,
      children: [
        DefaultTextStyle.merge(
          style: theme.typography.body.sm.copyWith(
            fontWeight: FontWeight.w600,
            color: theme.colors.foreground,
          ),
          child: label!,
        ),
        const SizedBox(height: 6),
        field,
      ],
    );
  }
}
