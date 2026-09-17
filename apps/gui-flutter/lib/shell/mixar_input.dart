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
