import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// Rounded option card shared by the setup steps that present one decision:
/// the detected-controller row and the analytics consent.
///
/// A card fill, the dialog corner and a hairline border keep it on the stage;
/// [highlighted] swaps the border for the accent at double weight, matching
/// the selected state used elsewhere.
class OnboardingOptionCard extends StatelessWidget {
  const new({
    required this.icon,
    required this.title,
    required this.trailing,
    this.subtitle,
    this.highlighted = false,
    super.key,
  });

  final IconData icon;
  final String title;
  final String? subtitle;

  /// Control at the end of the row (a button or a switch).
  final Widget trailing;
  final bool highlighted;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: theme.colors.card,
        borderRadius: theme.style.borderRadius.dialog,
        border: Border.all(
          color: highlighted ? theme.colors.primary : theme.colors.border,
          width: highlighted ? 2 : theme.style.borderWidth,
        ),
      ),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          spacing: 16,
          children: [
            Icon(
              icon,
              size: 20,
              color: highlighted
                  ? theme.colors.primary
                  : theme.colors.mutedForeground,
            ),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                spacing: 4,
                children: [
                  Text(
                    title,
                    style: theme.typography.body.lg.copyWith(
                      fontWeight: FontWeight.w600,
                      height: 1.25,
                    ),
                  ),
                  if (subtitle case final text?)
                    Text(
                      text,
                      style: theme.typography.body.sm.copyWith(
                        color: theme.colors.mutedForeground,
                        height: 1.4,
                      ),
                    ),
                ],
              ),
            ),
            trailing,
          ],
        ),
      ),
    );
  }
}
