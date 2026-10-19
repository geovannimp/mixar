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

  /// Validation message. Renders below the field and puts the border in its
  /// error state, the same way [ShadInputFormField] reports errors.
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
      decoration: _decoration(),
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
    final hasError = errorText != null;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      mainAxisSize: MainAxisSize.min,
      children: [
        if (label != null) ...[
          DefaultTextStyle.merge(
            style: theme.typography.body.sm.copyWith(
              fontWeight: FontWeight.w600,
              color: hasError
                  ? theme.colors.destructive
                  : theme.colors.foreground,
            ),
            child: label!,
          ),
          const SizedBox(height: 6),
        ],
        field,
        if (hasError) ...[
          const SizedBox(height: 6),
          DefaultTextStyle.merge(
            style: theme.typography.body.sm.copyWith(
              fontWeight: FontWeight.w500,
              color: theme.colors.destructive,
            ),
            child: Text(errorText!),
          ),
        ],
      ],
    );
  }

  /// Borderless fields stay flat; an error state merges into the existing
  /// decoration so the invalid field still reads as invalid without losing
  /// the caller's border intent.
  ShadDecoration? _decoration() {
    if (errorText == null) {
      return borderless ? ShadDecoration.none : null;
    }
    final base = borderless ? ShadDecoration.none : const ShadDecoration();
    return base.merge(const ShadDecoration(hasError: true));
  }
}
