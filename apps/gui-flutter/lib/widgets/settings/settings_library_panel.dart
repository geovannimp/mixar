import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/utils/key_format.dart';
import 'package:gui_flutter/core/utils/library_row_fields.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_checkbox.dart';
import 'package:gui_flutter/widgets/settings/settings_field.dart';
import 'package:gui_flutter/widgets/settings/settings_header_switch.dart';
import 'package:gui_flutter/widgets/settings/settings_section_body.dart';
import 'package:gui_flutter/widgets/settings/settings_section_card.dart';
import 'package:gui_flutter/widgets/settings/settings_select.dart';
import 'package:gui_flutter/widgets/settings/settings_visual_choice.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

class SettingsLibraryPanel extends StatelessWidget {
  const new({required this.draft, required this.onChanged, super.key});

  final AppSettings draft;
  final ValueChanged<AppSettings> onChanged;

  static const _stemsFormats = ['opus', 'flac'];
  static const _keyDisplayModes = KeyDisplayModeSetting.values;
  static const _keyColorModes = KeyColorModeSetting.values;

  static String _normalizedStemsFormat(String format) =>
      _stemsFormats.contains(format) ? format : 'opus';

  static String _normalizedStemsModel(String model) =>
      kStemModels.contains(model) ? model : kDefaultStemModel;

  static String _stemsModelLabel(String model) => switch (model) {
    'scnet_int8_v1' => 'SCNet (int8)',
    _ => 'HTDemucs',
  };

  static String _keyDisplayLabel(
    AppLocalizations l10n,
    KeyDisplayModeSetting mode,
  ) => switch (mode) {
    KeyDisplayModeSetting.musical => l10n.settingsLibraryKeyDisplayMusical,
    KeyDisplayModeSetting.camelot => l10n.settingsLibraryKeyDisplayCamelot,
  };

  static String _keyDisplaySubtitle(
    AppLocalizations l10n,
    KeyDisplayModeSetting mode,
  ) => switch (mode) {
    KeyDisplayModeSetting.musical =>
      l10n.settingsLibraryKeyDisplayMusicalSubtitle,
    KeyDisplayModeSetting.camelot =>
      l10n.settingsLibraryKeyDisplayCamelotSubtitle,
  };

  static String _keyColorLabel(
    AppLocalizations l10n,
    KeyColorModeSetting mode,
  ) => switch (mode) {
    KeyColorModeSetting.off => l10n.settingsLibraryKeyColorOff,
    KeyColorModeSetting.absolute => l10n.settingsLibraryKeyColorAbsolute,
    KeyColorModeSetting.harmonic => l10n.settingsLibraryKeyColorHarmonic,
  };

  static String _keyColorSubtitle(
    AppLocalizations l10n,
    KeyColorModeSetting mode,
  ) => switch (mode) {
    KeyColorModeSetting.off => l10n.settingsLibraryKeyColorOffSubtitle,
    KeyColorModeSetting.absolute =>
      l10n.settingsLibraryKeyColorAbsoluteSubtitle,
    KeyColorModeSetting.harmonic =>
      l10n.settingsLibraryKeyColorHarmonicSubtitle,
  };

  static String _stemsFormatLabel(AppLocalizations l10n, String format) =>
      switch (format) {
        'flac' => l10n.settingsLibraryStemFlac,
        _ => l10n.settingsLibraryStemOpus,
      };

  static String _rowFieldLabel(
    AppLocalizations l10n,
    LibraryRowFieldSetting field,
  ) => switch (field) {
    LibraryRowFieldSetting.album => l10n.settingsLibraryFieldAlbum,
    LibraryRowFieldSetting.genre => l10n.settingsLibraryFieldGenre,
    LibraryRowFieldSetting.year => l10n.settingsLibraryFieldYear,
    LibraryRowFieldSetting.trackNumber => l10n.settingsLibraryFieldTrackNumber,
    LibraryRowFieldSetting.discNumber => l10n.settingsLibraryFieldDiscNumber,
    LibraryRowFieldSetting.comment => l10n.settingsLibraryFieldComment,
    LibraryRowFieldSetting.isrc => l10n.settingsLibraryFieldIsrc,
    LibraryRowFieldSetting.duration => l10n.settingsLibraryFieldDuration,
    LibraryRowFieldSetting.rating => l10n.settingsLibraryFieldRating,
    LibraryRowFieldSetting.color => l10n.settingsLibraryFieldColor,
    LibraryRowFieldSetting.bpm => l10n.settingsLibraryFieldBpm,
    LibraryRowFieldSetting.key => l10n.settingsLibraryFieldKey,
  };

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;

    return SettingsSectionBody(
      title: l10n.settingsSectionLibrary,
      description: l10n.settingsLibraryDescription,
      cards: [
        SettingsSectionCard(
          title: l10n.settingsLibraryTagWriteBackTitle,
          subtitle: l10n.settingsLibraryTagWriteBackDescription,
          trailing: SettingsHeaderSwitch(
            value: draft.tagWriteBack,
            semanticsLabel: l10n.settingsLibraryTagWriteBack,
            onChanged: (enabled) =>
                onChanged(copyAppSettings(draft, tagWriteBack: enabled)),
          ),
        ),
        SettingsSectionCard(
          title: l10n.settingsLibraryMusicalKeyTitle,
          subtitle: l10n.settingsLibraryMusicalKeyDescription,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            spacing: 16,
            children: [
              SettingsField(
                label: l10n.settingsLibraryKeyDisplayMode,
                child: SettingsVisualChoice<KeyDisplayModeSetting>(
                  value: draft.keyDisplayMode,
                  options: [
                    for (final mode in _keyDisplayModes)
                      SettingsVisualOption<KeyDisplayModeSetting>(
                        value: mode,
                        label: _keyDisplayLabel(l10n, mode),
                        subtitle: _keyDisplaySubtitle(l10n, mode),
                        preview: _KeyDisplayPreview(mode: mode),
                      ),
                  ],
                  onChanged: (m) =>
                      onChanged(copyAppSettings(draft, keyDisplayMode: m)),
                ),
              ),
              SettingsField(
                label: l10n.settingsLibraryKeyColorMode,
                child: SettingsSelect(
                  dialogTitle: l10n.settingsLibraryKeyColorMode,
                  value: draft.keyColorMode,
                  options: _keyColorModes,
                  labelBuilder: (mode) => _keyColorLabel(l10n, mode),
                  subtitleBuilder: (mode) => _keyColorSubtitle(l10n, mode),
                  onChanged: (m) =>
                      onChanged(copyAppSettings(draft, keyColorMode: m)),
                ),
              ),
            ],
          ),
        ),
        SettingsSectionCard(
          title: l10n.settingsLibraryListTitle,
          subtitle: l10n.settingsLibraryListDescription,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            spacing: 16,
            children: [
              SettingsCheckbox(
                label: l10n.settingsLibraryDimPlayedTracks,
                value: draft.dimPlayedTracks,
                onChanged: (enabled) =>
                    onChanged(copyAppSettings(draft, dimPlayedTracks: enabled)),
              ),
              SettingsField(
                label: l10n.settingsLibraryTrackRowLayout,
                child: SettingsSelect<LibraryRowDensitySetting>(
                  dialogTitle: l10n.settingsLibraryTrackRowLayout,
                  value: draft.libraryRowDensity,
                  options: const [
                    LibraryRowDensitySetting.comfortable,
                    LibraryRowDensitySetting.compact,
                  ],
                  labelBuilder: (mode) =>
                      mode == LibraryRowDensitySetting.compact
                      ? l10n.settingsLibraryRowCompact
                      : l10n.settingsLibraryRowComfortable,
                  subtitleBuilder: (mode) =>
                      mode == LibraryRowDensitySetting.compact
                      ? l10n.settingsLibraryRowCompactSubtitle
                      : l10n.settingsLibraryRowComfortableSubtitle,
                  onChanged: (mode) => onChanged(
                    copyAppSettings(draft, libraryRowDensity: mode),
                  ),
                ),
              ),
            ],
          ),
        ),
        // Chips under each list title, in the order they were turned on. Its
        // own card so twelve toggles do not bury the analysis and key controls.
        SettingsSectionCard(
          title: l10n.settingsLibraryVisibleFields,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            spacing: 10,
            children: [
              for (final field in LibraryRowFieldSetting.values)
                SettingsCheckbox(
                  label: _rowFieldLabel(l10n, field),
                  value: draft.libraryRowFields.contains(field),
                  onChanged: (shown) => onChanged(
                    copyAppSettings(
                      draft,
                      libraryRowFields: toggleLibraryRowField(
                        draft.libraryRowFields,
                        field,
                        shown: shown,
                      ),
                    ),
                  ),
                ),
            ],
          ),
        ),
        SettingsSectionCard(
          title: l10n.settingsLibraryStemsTitle,
          subtitle: l10n.settingsLibraryStemsDescription,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            spacing: 16,
            children: [
              SettingsField(
                label: l10n.settingsLibraryStemModel,
                child: SettingsSelect<String>(
                  dialogTitle: l10n.settingsLibraryStemModel,
                  value: _normalizedStemsModel(draft.stemsModel),
                  options: kStemModels,
                  labelBuilder: _stemsModelLabel,
                  onChanged: (v) =>
                      onChanged(copyAppSettings(draft, stemsModel: v)),
                ),
              ),
              SettingsField(
                label: l10n.settingsLibraryStemFormat,
                child: SettingsSelect<String>(
                  dialogTitle: l10n.settingsLibraryStemFormat,
                  value: _normalizedStemsFormat(draft.stemsFormat),
                  options: _stemsFormats,
                  labelBuilder: (format) => _stemsFormatLabel(l10n, format),
                  onChanged: (v) =>
                      onChanged(copyAppSettings(draft, stemsFormat: v)),
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }
}

/// The same key in each notation: `Am` for musical, `8A` for Camelot.
class _KeyDisplayPreview extends StatelessWidget {
  const new({required this.mode});

  final KeyDisplayModeSetting mode;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final label = formatDeckKey(
      '8A',
      mode == KeyDisplayModeSetting.musical
          ? KeyDisplayMode.musical
          : KeyDisplayMode.camelot,
    );
    return Center(
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
        decoration: BoxDecoration(
          color: theme.colors.muted,
          borderRadius: BorderRadius.circular(4),
          border: Border.all(color: theme.colors.border),
        ),
        child: Text(
          label,
          style: theme.typography.body.xs.copyWith(
            color: theme.colors.foreground,
            fontWeight: FontWeight.w600,
          ),
        ),
      ),
    );
  }
}
