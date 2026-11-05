import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/onboarding/onboarding_ui.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Step 5: the analytics consent as a plain either/or, defaulting to private.
class OnboardingPrivacyStep extends StatelessWidget {
  const new({required this.draft, required this.onChanged, super.key});

  final AppSettings draft;
  final ValueChanged<AppSettings> onChanged;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return OnboardingBody(
      eyebrow: l10n.onboardingKickerPrivacy,
      title: l10n.onboardingPrivacyTitle,
      description: l10n.onboardingPrivacyBody,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        spacing: 12,
        children: [
          _card(
            enabled: false,
            icon: LucideIcons.shieldCheck,
            title: l10n.onboardingPrivacyOffTitle,
            subtitle: l10n.onboardingPrivacyOffBody,
          ),
          _card(
            enabled: true,
            icon: LucideIcons.activity,
            title: l10n.onboardingPrivacyOnTitle,
            subtitle: l10n.onboardingPrivacyOnBody,
          ),
        ],
      ),
    );
  }

  /// One consent tile. [enabled] drives both its selected state and the value
  /// its tap writes, so the two tiles cannot disagree.
  Widget _card({
    required bool enabled,
    required IconData icon,
    required String title,
    required String subtitle,
  }) {
    return OnboardingChoiceCard(
      icon: icon,
      title: title,
      subtitle: subtitle,
      selected: draft.analyticsEnabled == enabled,
      onPress: () =>
          onChanged(copyAppSettings(draft, analyticsEnabled: enabled)),
    );
  }
}
