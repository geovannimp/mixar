import 'package:flutter/widgets.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/widgets/onboarding/onboarding_ui.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Last step: setup is done, here is how to start playing.
class OnboardingReadyStep extends StatelessWidget {
  const new({super.key});

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return OnboardingBody(
      eyebrow: l10n.onboardingKickerReady,
      title: l10n.onboardingReadyTitle,
      description: l10n.onboardingReadyBody,
      footnote: _ReadyFootnote(text: l10n.onboardingReadyFootnote),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        spacing: 12,
        children: [
          OnboardingTipRow(
            icon: LucideIcons.mousePointerClick,
            title: l10n.onboardingTipLoadTitle,
            subtitle: l10n.onboardingTipLoadBody,
          ),
          OnboardingTipRow(
            icon: LucideIcons.activity,
            title: l10n.onboardingTipAnalyzeTitle,
            subtitle: l10n.onboardingTipAnalyzeBody,
          ),
          OnboardingTipRow(
            icon: LucideIcons.settings2,
            title: l10n.onboardingTipSettingsTitle,
            subtitle: l10n.onboardingTipSettingsBody,
          ),
        ],
      ),
    );
  }
}

/// Closing note, called out: accent tint, accent rule and accent ink, so it
/// reads as the last thing to remember rather than a caption.
class _ReadyFootnote extends StatelessWidget {
  const new({required this.text});

  final String text;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final colors = theme.colors;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: colors.primaryTint,
        border: Border.all(color: colors.primary.withValues(alpha: 0.55)),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 11),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          spacing: 10,
          children: [
            Icon(LucideIcons.sparkles, size: 15, color: colors.primary),
            Flexible(
              child: Text(
                text,
                textAlign: TextAlign.center,
                style: theme.typography.body.sm.copyWith(
                  color: colors.foreground,
                  fontWeight: FontWeight.w600,
                  height: 1.4,
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
