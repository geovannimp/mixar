import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/core/utils/ui_layout.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_checkbox.dart';
import 'package:gui_flutter/widgets/settings/settings_field.dart';
import 'package:gui_flutter/widgets/settings/settings_header_switch.dart';
import 'package:gui_flutter/widgets/settings/settings_labels.dart';
import 'package:gui_flutter/widgets/settings/settings_preview_block.dart';
import 'package:gui_flutter/widgets/settings/settings_section_body.dart';
import 'package:gui_flutter/widgets/settings/settings_section_card.dart';
import 'package:gui_flutter/widgets/settings/settings_select.dart';
import 'package:gui_flutter/widgets/settings/settings_visual_choice.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// Settings → Layout: which mixer panels are visible, where the waveform sits,
/// and how the library opens.
class SettingsLayoutPanel extends StatelessWidget {
  const new({required this.draft, required this.onChanged, super.key});

  final AppSettings draft;
  final ValueChanged<AppSettings> onChanged;

  static String _presetLabel(AppLocalizations l10n, LayoutPreset preset) =>
      switch (preset) {
        LayoutPreset.desktop => l10n.settingsLayoutPresetDesktop,
        LayoutPreset.tablet => l10n.settingsLayoutPresetTablet,
        LayoutPreset.mobile => l10n.settingsLayoutPresetMobile,
        LayoutPreset.custom => l10n.settingsLayoutPresetCustom,
      };

  static String _presetSubtitle(AppLocalizations l10n, LayoutPreset preset) =>
      switch (preset) {
        LayoutPreset.desktop => l10n.settingsLayoutPresetDesktopSubtitle,
        LayoutPreset.tablet => l10n.settingsLayoutPresetTabletSubtitle,
        LayoutPreset.mobile => l10n.settingsLayoutPresetMobileSubtitle,
        LayoutPreset.custom => l10n.settingsLayoutPresetCustomSubtitle,
      };

  static String _positionLabel(
    AppLocalizations l10n,
    WaveformPositionSetting value,
  ) => switch (value) {
    WaveformPositionSetting.top => l10n.settingsLayoutWaveformTop,
    WaveformPositionSetting.middleHorizontal =>
      l10n.settingsLayoutWaveformMiddleHorizontal,
    WaveformPositionSetting.middleVertical =>
      l10n.settingsLayoutWaveformMiddleVertical,
    WaveformPositionSetting.bottom => l10n.settingsLayoutWaveformBottom,
  };

  static String _drawerSideLabel(
    AppLocalizations l10n,
    LibraryDrawerSideSetting side,
  ) => switch (side) {
    LibraryDrawerSideSetting.left => l10n.settingsLayoutLibraryDrawerLeft,
    LibraryDrawerSideSetting.right => l10n.settingsLayoutLibraryDrawerRight,
    LibraryDrawerSideSetting.top => l10n.settingsLayoutLibraryDrawerTop,
    LibraryDrawerSideSetting.bottom => l10n.settingsLayoutLibraryDrawerBottom,
  };

  /// A labelled select: in this panel the field label is also the dialog title,
  /// so every select wires the same two strings.
  Widget _labeledSelect<T>({
    required String label,
    required T value,
    required List<T> options,
    required String Function(T value) labelBuilder,
    required ValueChanged<T> onChanged,
    String Function(T value)? subtitleBuilder,
    String? hint,
  }) => SettingsField(
    label: label,
    hint: hint,
    child: SettingsSelect<T>(
      dialogTitle: label,
      value: value,
      options: options,
      labelBuilder: labelBuilder,
      subtitleBuilder: subtitleBuilder,
      onChanged: onChanged,
    ),
  );

  /// Header-switch card: title, subtitle, the toggle, and an optional body.
  ///
  /// The panel shows four of these (mixer strip, FX bar, waveform, drawer) and
  /// they are all the same shape, so a change to the shape lands in one place.
  SettingsSectionCard _toggleCard({
    required String title,
    required String subtitle,
    required bool value,
    required String semanticsLabel,
    required ValueChanged<bool> onChanged,
    Widget? child,
    List<String> searchTerms = const [],
  }) => SettingsSectionCard(
    title: title,
    subtitle: subtitle,
    searchTerms: searchTerms,
    trailing: SettingsHeaderSwitch(
      value: value,
      semanticsLabel: semanticsLabel,
      onChanged: onChanged,
    ),
    child: child,
  );

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final preset = layoutPresetFor(draft);
    // The crossfader rides at the foot of whichever center column exists: under
    // the mixer channels, or under a middle waveform that took the column. Its
    // switch stays reachable while either column is on screen. The meters are
    // part of the strip's channels, so they drop out whenever the strip does —
    // switched off, or pushed out of the row by a middle waveform.
    final centeredWaveform =
        draft.showWaveform &&
        (draft.waveformPosition == WaveformPositionSetting.middleHorizontal ||
            draft.waveformPosition == WaveformPositionSetting.middleVertical);
    final hasCrossfader = draft.showMixer || centeredWaveform;
    return SettingsSectionBody(
      title: l10n.settingsSectionLayout,
      description: l10n.settingsLayoutDescription,
      cards: [
        SettingsSectionCard(
          title: l10n.settingsLayoutPresetTitle,
          subtitle: l10n.settingsLayoutPresetDescription,
          child: _labeledSelect<LayoutPreset>(
            label: l10n.settingsLayoutPreset,
            hint: preset == LayoutPreset.custom
                ? l10n.settingsLayoutPresetCustomSubtitle
                : null,
            value: preset,
            // `custom` is a result, not a choice: it only ever shows as the
            // collapsed value once the toggles below stop matching a preset.
            options: kLayoutPresets,
            labelBuilder: (value) => _presetLabel(l10n, value),
            subtitleBuilder: (value) => _presetSubtitle(l10n, value),
            onChanged: (value) => onChanged(applyLayoutPreset(draft, value)),
          ),
        ),
        SettingsSectionCard(
          title: l10n.settingsLayoutDeckCount,
          subtitle: l10n.settingsLayoutDeckCountDescription,
          child: _labeledSelect<int>(
            label: l10n.settingsLayoutDeckCountDecks,
            value: draft.numDecks,
            options: const [2, 4],
            labelBuilder: (count) => count == 2
                ? l10n.settingsLayoutDeckCountTwo
                : l10n.settingsLayoutDeckCountFour,
            onChanged: (count) => onChanged(
              normalizeAppSettings(copyAppSettings(draft, numDecks: count)),
            ),
          ),
        ),
        _toggleCard(
          title: l10n.settingsLayoutMixerTitle,
          subtitle: l10n.settingsLayoutMixerDescription,
          value: draft.showMixer,
          semanticsLabel: l10n.settingsLayoutMixerEnable,
          onChanged: (value) =>
              onChanged(copyAppSettings(draft, showMixer: value)),
          // The crossfader outlives the strip: a middle waveform keeps the
          // column and its foot, so the switch stays here whenever one of the
          // two columns is on screen. The meters belong to the strip itself.
          child: hasCrossfader
              ? Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  spacing: 16,
                  children: [
                    SettingsCheckbox(
                      label: l10n.settingsLayoutShowCrossfader,
                      value: draft.showCrossfader,
                      onChanged: (value) => onChanged(
                        copyAppSettings(draft, showCrossfader: value),
                      ),
                    ),
                  ],
                )
              : null,
        ),
        _toggleCard(
          title: l10n.settingsLayoutFxBarTitle,
          subtitle: l10n.settingsLayoutFxBarDescription,
          value: draft.showFxBar,
          semanticsLabel: l10n.settingsLayoutFxBarEnable,
          onChanged: (value) =>
              onChanged(copyAppSettings(draft, showFxBar: value)),
        ),
        _toggleCard(
          title: l10n.settingsLayoutWaveformTitle,
          subtitle: l10n.settingsLayoutWaveformDescription,
          value: draft.showWaveform,
          semanticsLabel: l10n.settingsLayoutShowWaveform,
          onChanged: (show) =>
              onChanged(copyAppSettings(draft, showWaveform: show)),
          // The position only matters while something is drawn.
          child: draft.showWaveform
              ? SettingsField(
                  label: l10n.settingsLayoutWaveformPosition,
                  child: SettingsVisualChoice<WaveformPositionSetting>(
                    value: draft.waveformPosition,
                    axis: Axis.horizontal,
                    previewAspectRatio: 16 / 9,
                    options: [
                      for (final value in WaveformPositionSetting.values)
                        SettingsVisualOption<WaveformPositionSetting>(
                          value: value,
                          label: _positionLabel(l10n, value),
                          preview: _WaveformPositionPreview(position: value),
                        ),
                    ],
                    onChanged: (position) => onChanged(
                      copyAppSettings(draft, waveformPosition: position),
                    ),
                  ),
                )
              : null,
        ),
        _toggleCard(
          title: l10n.settingsLayoutLibraryDrawerTitle,
          subtitle: l10n.settingsLayoutLibraryDrawerDescription,
          value: draft.libraryDrawerEnabled,
          semanticsLabel: l10n.settingsLayoutLibraryDrawerEnable,
          onChanged: (value) =>
              onChanged(copyAppSettings(draft, libraryDrawerEnabled: value)),
          child: draft.libraryDrawerEnabled
              ? SettingsField(
                  label: l10n.settingsLayoutLibraryDrawerPosition,
                  child: SettingsVisualChoice<LibraryDrawerSideSetting>(
                    value: draft.libraryDrawerSide,
                    axis: Axis.horizontal,
                    previewAspectRatio: 16 / 9,
                    options: [
                      for (final value in LibraryDrawerSideSetting.values)
                        SettingsVisualOption<LibraryDrawerSideSetting>(
                          value: value,
                          label: _drawerSideLabel(l10n, value),
                          preview: _LibraryDrawerSidePreview(side: value),
                        ),
                    ],
                    onChanged: (side) => onChanged(
                      copyAppSettings(draft, libraryDrawerSide: side),
                    ),
                  ),
                )
              : null,
        ),
        SettingsSectionCard(
          title: l10n.settingsUiSelectStyle,
          child: SettingsSelect<SelectStyleSetting>(
            dialogTitle: l10n.settingsUiSelectStyle,
            value: draft.selectStyle,
            options: const [
              SelectStyleSetting.auto,
              SelectStyleSetting.desktop,
              SelectStyleSetting.mobile,
            ],
            labelBuilder: (mode) => selectStyleSettingLabel(l10n, mode),
            subtitleBuilder: (mode) => selectStyleSettingSubtitle(l10n, mode),
            onChanged: (mode) =>
                onChanged(copyAppSettings(draft, selectStyle: mode)),
          ),
        ),
      ],
    );
  }
}

/// Miniature of the deck row with the waveform band in its chosen slot.
class _WaveformPositionPreview extends StatelessWidget {
  const new({required this.position});

  final WaveformPositionSetting position;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final block = colors.mutedForeground.withValues(alpha: 0.35);
    final accent = colors.primary;
    final soft = colors.primary.withValues(alpha: 0.38);

    Widget sized(int flex, Color color) => Expanded(
      flex: flex,
      child: SettingsPreviewBlock(color: color),
    );
    Widget band(double height, Color color) => SizedBox(
      height: height,
      child: SettingsPreviewBlock(color: color),
    );
    Widget deckRow() => Expanded(
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        spacing: 2,
        children: [sized(3, block), sized(2, soft), sized(3, block)],
      ),
    );
    Widget stackedLanes() => Expanded(
      flex: 4,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        spacing: 2,
        children: [
          Expanded(child: SettingsPreviewBlock(color: accent)),
          Expanded(child: SettingsPreviewBlock(color: accent)),
        ],
      ),
    );
    Widget sideBySideLanes() => Expanded(
      flex: 4,
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        spacing: 2,
        children: [
          Expanded(child: SettingsPreviewBlock(color: accent)),
          Expanded(child: SettingsPreviewBlock(color: accent)),
        ],
      ),
    );

    return switch (position) {
      WaveformPositionSetting.top => Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        spacing: 2,
        children: [band(6, accent), deckRow()],
      ),
      WaveformPositionSetting.bottom => Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        spacing: 2,
        children: [deckRow(), band(6, accent)],
      ),
      WaveformPositionSetting.middleHorizontal => Row(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        spacing: 2,
        children: [sized(3, block), stackedLanes(), sized(3, block)],
      ),
      WaveformPositionSetting.middleVertical => Row(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        spacing: 2,
        children: [sized(3, block), sideBySideLanes(), sized(3, block)],
      ),
    };
  }
}

/// Miniature of the pane with the library band on the chosen edge.
class _LibraryDrawerSidePreview extends StatelessWidget {
  const new({required this.side});

  final LibraryDrawerSideSetting side;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final block = colors.mutedForeground.withValues(alpha: 0.35);
    final accent = colors.primary;

    Widget body() => Expanded(child: SettingsPreviewBlock(color: block));
    Widget edge({double? width, double? height}) => SizedBox(
      width: width,
      height: height,
      child: SettingsPreviewBlock(color: accent),
    );

    return switch (side) {
      LibraryDrawerSideSetting.left => Row(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        spacing: 2,
        children: [edge(width: 14), body()],
      ),
      LibraryDrawerSideSetting.right => Row(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        spacing: 2,
        children: [body(), edge(width: 14)],
      ),
      LibraryDrawerSideSetting.top => Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        spacing: 2,
        children: [edge(height: 7), body()],
      ),
      LibraryDrawerSideSetting.bottom => Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        spacing: 2,
        children: [body(), edge(height: 7)],
      ),
    };
  }
}
