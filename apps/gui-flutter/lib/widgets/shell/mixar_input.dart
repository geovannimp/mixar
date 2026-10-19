import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
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
    this.errorText,
    this.borderless = false,
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

  /// Validation message. Renders below the field and requests the field's
  /// error border. This widget draws its own label and message instead of
  /// using `ShadInputDecorator`, so the presentation mirrors
  /// [ShadInputFormField]'s intent rather than sharing its implementation.
  final String? errorText;

  /// Overrides [ShadInput]'s theme padding. Used by dense toolbars that need a
  /// shorter field than the default vertical padding produces.
  final EdgeInsetsGeometry? padding;

  /// Drops [ShadInput]'s rounded border, for a flat field that fills a bar.
  ///
  /// Normally paired with a compact [padding]: [borderless] alone keeps the
  /// theme's default vertical padding, which is taller than a dense bar wants.
  final bool borderless;
  final int? maxLines;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    // An empty message is not an error state; only non-empty text is.
    final error = (errorText == null || errorText!.isEmpty) ? null : errorText;
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
      decoration: _decoration(error != null),
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
    // Label-less and error-free is the shape this widget had before it grew
    // error support — returning the field directly keeps those callers (dense
    // toolbars) out of a wrapping Column they never had.
    if (label == null && error == null) {
      return field;
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      mainAxisSize: MainAxisSize.min,
      children: [
        if (label != null) ...[
          DefaultTextStyle.merge(
            style: theme.typography.body.sm.copyWith(
              fontWeight: FontWeight.w600,
              color: error != null
                  ? theme.colors.destructive
                  : theme.colors.foreground,
            ),
            child: label!,
          ),
          const SizedBox(height: 6),
        ],
        field,
        if (error != null) ...[
          const SizedBox(height: 6),
          DefaultTextStyle.merge(
            style: theme.typography.body.sm.copyWith(
              fontWeight: FontWeight.w500,
              color: theme.colors.destructive,
            ),
            child: Text(error),
          ),
        ],
      ],
    );
  }

  /// Decoration for the current field state.
  ///
  /// - No error: `null` for a normal field (the theme's input decoration is
  ///   merged in by `ShadInput` itself), `ShadDecoration.none` when borderless
  ///   — so nothing about a caller's border intent is ever replaced wholesale.
  /// - Error: only the `hasError` flag is merged on, which makes `ShadInput`
  ///   paint its error border on a normal field. A borderless field keeps
  ///   `ShadBorder.none` (`none.canMerge` is false, so it wins the merge and
  ///   stays flat): its invalid state reads through the red label and the
  ///   message [MixarInput] renders below, not through a border.
  ShadDecoration? _decoration(bool hasError) {
    if (!hasError) {
      return borderless ? ShadDecoration.none : null;
    }
    return borderless
        ? ShadDecoration.none.merge(const ShadDecoration(hasError: true))
        : const ShadDecoration(hasError: true);
  }
}
