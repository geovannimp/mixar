import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/onboarding/onboarding_ui.dart';
import 'package:gui_flutter/widgets/settings/settings_labels.dart';
import 'package:gui_flutter/widgets/settings/settings_select.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Key the widget tests open the language choice with.
const ValueKey<String> kOnboardingLanguageSelectKey = ValueKey(
  'onboarding-language-select',
);

/// Step 2: the interface language, on its own so it is one decision.
///
/// Picking a language repaints the whole setup immediately: `Application`
/// reads the draft, so the rest of the flow is already in the chosen language
/// by the next page.
class OnboardingLanguageStep extends StatelessWidget {
  const new({required this.draft, required this.onChanged, super.key});

  final AppSettings draft;
  final ValueChanged<AppSettings> onChanged;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return OnboardingBody(
      eyebrow: l10n.onboardingKickerLanguage,
      icon: LucideIcons.languages,
      title: l10n.onboardingLanguageTitle,
      description: l10n.onboardingLanguageBody,
      child: SettingsSelect<UiLanguageSetting>(
        key: kOnboardingLanguageSelectKey,
        value: draft.uiLanguage,
        options: UiLanguageSetting.values,
        dialogTitle: l10n.onboardingLanguageTitle,
        labelBuilder: (language) => uiLanguageSettingLabel(l10n, language),
        subtitleBuilder: (language) => language == UiLanguageSetting.system
            ? l10n.onboardingLanguageFollowsOs
            : null,
        onChanged: (language) =>
            onChanged(copyAppSettings(draft, uiLanguage: language)),
      ),
    );
  }
}
