import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/theme/mixar_button_style.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_checkbox.dart';
import 'package:gui_flutter/widgets/settings/settings_field.dart';
import 'package:gui_flutter/widgets/settings/settings_labels.dart';
import 'package:gui_flutter/widgets/settings/settings_section_body.dart';
import 'package:gui_flutter/widgets/settings/settings_section_card.dart';
import 'package:gui_flutter/widgets/settings/settings_select.dart';
import 'package:gui_flutter/widgets/settings/settings_visual_choice.dart';
import 'package:gui_flutter/widgets/settings/theme_swatch.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

class SettingsUiPanel extends StatelessWidget {
  const new({
    required this.draft,
    required this.onChanged,
    required this.savedAccentPalette,
    super.key,
  });

  final AppSettings draft;
  final ValueChanged<AppSettings> onChanged;

  /// Accent the app is running with, before this draft is saved.
  ///
  /// The theme previews paint the mixed window in this palette so the accent
  /// they show only switches after a save; the picker below still previews the
  /// draft's own swatch.
  final AccentPaletteSetting savedAccentPalette;

  static const List<UiLanguageSetting> _languages = UiLanguageSetting.values;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return SettingsSectionBody(
      title: l10n.settingsSectionUi,
      description: l10n.settingsUiDescription,
      cards: [
        SettingsSectionCard(
          title: l10n.settingsUiChromeTitle,
          subtitle: l10n.settingsUiChromeDescription,
          searchTerms: [
            l10n.settingsUiLanguageLabel,
            l10n.settingsShowTooltips,
          ],
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            spacing: 16,
            children: [
              SettingsField(
                label: l10n.settingsUiLanguageLabel,
                child: SettingsSelect<UiLanguageSetting>(
                  value: draft.uiLanguage,
                  options: _languages,
                  labelBuilder: (value) => uiLanguageSettingLabel(l10n, value),
                  onChanged: (language) =>
                      onChanged(copyAppSettings(draft, uiLanguage: language)),
                ),
              ),
              SettingsCheckbox(
                label: l10n.settingsShowTooltips,
                value: draft.showTooltips,
                onChanged: (enabled) =>
                    onChanged(copyAppSettings(draft, showTooltips: enabled)),
              ),
            ],
          ),
        ),
        SettingsSectionCard(
          title: l10n.settingsUiThemeTitle,
          subtitle: l10n.settingsUiThemeDescription,
          child: SettingsField(
            label: l10n.settingsUiTheme,
            child: SettingsVisualChoice<ThemeSetting>(
              value: draft.theme,
              axis: Axis.horizontal,
              previewAspectRatio: 16 / 9,
              options: [
                for (final theme in const [
                  ThemeSetting.system,
                  ThemeSetting.light,
                  ThemeSetting.dark,
                ])
                  SettingsVisualOption<ThemeSetting>(
                    value: theme,
                    label: themeSettingLabel(l10n, theme),
                    preview: ThemeSwatch(
                      value: theme,
                      palette: accentPaletteFromSettings(savedAccentPalette),
                    ),
                  ),
              ],
              onChanged: (theme) =>
                  onChanged(copyAppSettings(draft, theme: theme)),
            ),
          ),
        ),
        SettingsSectionCard(
          title: l10n.settingsUiAccentColor,
          subtitle: l10n.settingsUiAccentDescription,
          child: SettingsVisualChoice<AccentPaletteSetting>(
            value: draft.accentPalette,
            axis: Axis.horizontal,
            options: [
              for (final palette in const [
                AccentPaletteSetting.teal,
                AccentPaletteSetting.blue,
                AccentPaletteSetting.violet,
                AccentPaletteSetting.orange,
                AccentPaletteSetting.pink,
              ])
                SettingsVisualOption<AccentPaletteSetting>(
                  value: palette,
                  label: accentPaletteLabel(l10n, palette),
                  preview: _AccentSwatch(palette: palette),
                ),
            ],
            onChanged: (palette) =>
                onChanged(copyAppSettings(draft, accentPalette: palette)),
          ),
        ),
        SettingsSectionCard(
          title: l10n.settingsUiOnboardingTitle,
          subtitle: l10n.settingsUiOnboardingDescription,
          trailing: AppButton(
            size: MixarButtonSize.sm,
            variant: MixarButtonVariant.outline,
            onPress: () =>
                onChanged(copyAppSettings(draft, onboardingCompleted: false)),
            child: Text(l10n.settingsUiOnboardingReplay),
          ),
        ),
      ],
    );
  }
}

/// The palette's own mid tone, centred above its label.
class _AccentSwatch extends StatelessWidget {
  const new({required this.palette});

  final AccentPaletteSetting palette;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return Center(
      child: SizedBox.square(
        dimension: 24,
        child: DecoratedBox(
          decoration: BoxDecoration(
            shape: BoxShape.circle,
            color: accentPaletteFromSettings(palette).scale.s500,
            border: Border.all(color: theme.colors.border),
          ),
        ),
      ),
    );
  }
}
