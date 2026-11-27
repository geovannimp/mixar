import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/onboarding/onboarding_ui.dart';
import 'package:gui_flutter/widgets/settings/settings_header_switch.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Step 5: the analytics consent as a single switch, defaulting to private.
class OnboardingPrivacyStep extends StatelessWidget {
  const new({required this.draft, required this.onChanged, super.key});

  final AppSettings draft;
  final ValueChanged<AppSettings> onChanged;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final enabled = draft.analyticsEnabled;
    return OnboardingBody(
      eyebrow: l10n.onboardingKickerPrivacy,
      icon: LucideIcons.shieldCheck,
      title: l10n.onboardingPrivacyTitle,
      description: l10n.onboardingPrivacyBody,
      child: OnboardingOptionCard(
        // One fixed copy and glyph: the switch, not the card, carries the
        // choice, so the row never rewrites itself under the pointer.
        icon: LucideIcons.shieldCheck,
        title: l10n.settingsAnalyticsTitle,
        subtitle: l10n.settingsAnalyticsSubtitle,
        highlighted: enabled,
        trailing: SettingsHeaderSwitch(
          value: enabled,
          semanticsLabel: l10n.settingsAnalyticsToggle,
          onChanged: (value) =>
              onChanged(copyAppSettings(draft, analyticsEnabled: value)),
        ),
      ),
    );
  }
}
