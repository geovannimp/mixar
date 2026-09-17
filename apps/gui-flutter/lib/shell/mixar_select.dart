import 'package:flutter/widgets.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:shadcn_ui/shadcn_ui.dart';

/// Mixar select / dropdown over [ShadSelect].
class MixarSelect<T> extends StatelessWidget {
  const new({
    required this.value,
    required this.options,
    required this.labelBuilder,
    required this.onChanged,
    super.key,
    this.subtitleBuilder,
    this.enabled = true,
    this.placeholder,
  });

  final T value;
  final List<T> options;
  final String Function(T value) labelBuilder;
  final String Function(T value)? subtitleBuilder;
  final ValueChanged<T> onChanged;
  final bool enabled;
  final Widget? placeholder;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    // ShadSelect sizes to content unless minWidth is set; match Forui full-bleed.
    return LayoutBuilder(
      builder: (context, constraints) {
        final fill = constraints.maxWidth.isFinite
            ? constraints.maxWidth
            : null;
        return ShadSelect<T>(
          initialValue: value,
          enabled: enabled,
          placeholder: placeholder,
          minWidth: fill,
          onChanged: (next) {
            if (next != null) {
              onChanged(next);
            }
          },
          selectedOptionBuilder: (context, selected) =>
              Text(labelBuilder(selected)),
          options: [
            for (final option in options)
              ShadOption(
                value: option,
                child: subtitleBuilder == null
                    ? Text(labelBuilder(option))
                    : Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          Text(labelBuilder(option)),
                          Text(
                            subtitleBuilder!(option),
                            style: theme.typography.body.xs.copyWith(
                              color: theme.colors.mutedForeground,
                            ),
                          ),
                        ],
                      ),
              ),
          ],
        );
      },
    );
  }
}
