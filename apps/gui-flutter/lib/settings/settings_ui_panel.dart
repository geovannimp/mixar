import 'package:flutter/widgets.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_field.dart';
import 'package:gui_flutter/settings/settings_widgets.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';

class SettingsUiPanel extends StatelessWidget {
  const new({required this.draft, required this.onChanged, super.key});

  final AppSettings draft;
  final ValueChanged<AppSettings> onChanged;

  static const List<UiLanguageSetting> _languages = UiLanguageSetting.values;

  static String _languageLabel(
    AppLocalizations l10n,
    UiLanguageSetting value,
  ) => switch (value) {
    UiLanguageSetting.system => l10n.settingsLanguageSystemDefault,
    UiLanguageSetting.en => 'English',
    UiLanguageSetting.ptBr => 'Português (Brasil)',
  };

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      spacing: 16,
      children: [
SettingsSectionHeader(
          title: l10n.settingsSectionUi,
          description: l10n.settingsUiDescription,
        ),
        SettingsPanel(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            spacing: 16,
            children: [
              SettingsField(
                label: l10n.settingsUiLanguageLabel,
                child: SettingsSelect<UiLanguageSetting>(
                  value: draft.uiLanguage,
                  options: _languages,
                  labelBuilder: (value) => _languageLabel(l10n, value),
                  onChanged: (language) => onChanged(
                    copyAppSettings(draft, uiLanguage: language),
                  ),
                ),
              ),
              SettingsToggle(
                label: l10n.settingsShowTooltips,
                value: draft.showTooltips,
                onChanged: (enabled) =>
                    onChanged(copyAppSettings(draft, showTooltips: enabled)),
              ),
              SettingsField(
                label: 'Select style',
                child: SettingsSelect<SelectStyleSetting>(
                  dialogTitle: 'Select style',
                  value: draft.selectStyle,
                  options: const [
                    SelectStyleSetting.auto,
                    SelectStyleSetting.desktop,
                    SelectStyleSetting.mobile,
                  ],
                  labelBuilder: (mode) => switch (mode) {
                    SelectStyleSetting.auto => 'Auto',
                    SelectStyleSetting.desktop => 'Desktop',
                    SelectStyleSetting.mobile => 'Mobile',
                  },
                  subtitleBuilder: (mode) => switch (mode) {
                    SelectStyleSetting.auto =>
                      'Platform default — desktop OS uses popovers, '
                          'phones use dialogs.',
                    SelectStyleSetting.desktop =>
                      'Anchored popovers for selects and ⋯ menus.',
                    SelectStyleSetting.mobile =>
                      'Open selects and ⋯ menus in a dialog.',
                  },
                  onChanged: (mode) =>
                      onChanged(copyAppSettings(draft, selectStyle: mode)),
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }
}

⚠ 1 unresolved conflict detected
- ours = HEAD
- theirs = aefe1e47 (feat(gui): localize Settings strings for en and pt_BR)
NOTICE: Inspect a block by reading `conflict://<N>` (add `/ours` / `/theirs` / `/base` to render a single side). Resolve with `write({ path: "conflict://<N>", content })`, or bulk-resolve every registered conflict with `write({ path: "conflict://*", content })`. Writes replace ONLY the marker block (markers + all sides) — never repeat the lines before/after it; they stay in place.
`content` shorthand: a line that is exactly `@ours` / `@theirs` / `@base` / `@both` expands to that recorded section. `@both` is ours-then-theirs with no separator — only for additive conflicts where each side adds something different; NEVER for competing edits of the same lines (pick a side or write the combined text). Lines that are not a token pass through verbatim, so `"// keep both\n@ours\n@theirs"` literally writes the comment, then ours, then theirs.
Per-id bulk: `write({ path: "conflict://*", content: "1: @ours\n2: @theirs\n…" })` resolves each listed id with that side in ONE call — the cheapest way through many pick-one conflicts; unlisted ids stay registered.
Resolve each block faithfully: keep one side (`@ours`/`@theirs`), or combine them when both intents apply — never invent content beyond the recorded sides, and never stack both sides of competing edits. Resolve several conflicts in a single turn by issuing multiple `write` calls at once; ids stay valid as earlier blocks are resolved.

──── #8  L32-40 ────
<<< ours
        const SettingsSectionHeader(
          title: 'UI',
          description: 'Chrome, hover tips, and control presentation.',
>>> theirs
        SettingsSectionHeader(
          title: l10n.settingsSectionUi,
          description: l10n.settingsUiDescription,