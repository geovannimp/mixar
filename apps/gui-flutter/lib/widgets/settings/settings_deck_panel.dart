import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/core/utils/tempo_format.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_checkbox.dart';
import 'package:gui_flutter/widgets/settings/settings_field.dart';
import 'package:gui_flutter/widgets/settings/settings_header_switch.dart';
import 'package:gui_flutter/widgets/settings/settings_numeric_stepper.dart';
import 'package:gui_flutter/widgets/settings/settings_section_card.dart';
import 'package:gui_flutter/widgets/settings/settings_section_header.dart';
import 'package:gui_flutter/widgets/settings/settings_select.dart';

class SettingsDeckPanel extends ConsumerWidget {
  const new({required this.draft, required this.onChanged, super.key});

  final AppSettings draft;
  final ValueChanged<AppSettings> onChanged;

  static const List<JogModeSetting> _jogModes = JogModeSetting.values;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final banks = ref.watch(samplerBanksProvider).value ?? const [];
    final l10n = AppLocalizations.of(context)!;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      spacing: 16,
      children: [
        SettingsSectionHeader(
          title: l10n.settingsSectionDeck,
          description: l10n.settingsDeckDescription,
        ),
        SettingsSectionCard(
          title: l10n.settingsDeckCount,
          subtitle: l10n.settingsDeckLayoutDescription,
          child: SettingsField(
            label: l10n.settingsDeckLayoutDecks,
            child: SettingsSelect<int>(
              dialogTitle: l10n.settingsDeckLayoutDecks,
              value: draft.numDecks,
              options: const [2, 4],
              labelBuilder: (count) => count == 2
                  ? l10n.settingsDeckCountTwo
                  : l10n.settingsDeckCountFour,
              onChanged: (count) => onChanged(
                normalizeAppSettings(copyAppSettings(draft, numDecks: count)),
              ),
            ),
          ),
        ),
        SettingsSectionCard(
          title: l10n.settingsDeckJogTitle,
          subtitle: l10n.settingsDeckJogDescription,
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            spacing: 12,
            children: [
              Expanded(
                child: SettingsField(
                  label: l10n.settingsDeckTopJogMode,
                  child: SettingsSelect(
                    dialogTitle: l10n.settingsDeckTopJogMode,
                    value: draft.defaultTopJogMode,
                    options: _jogModes,
                    labelBuilder: (m) => _jogLabel(l10n, m),
                    onChanged: (m) =>
                        onChanged(copyAppSettings(draft, defaultTopJogMode: m)),
                  ),
                ),
              ),
              Expanded(
                child: SettingsField(
                  label: l10n.settingsDeckOuterJogMode,
                  child: SettingsSelect(
                    dialogTitle: l10n.settingsDeckOuterJogMode,
                    value: draft.defaultOuterJogMode,
                    options: _jogModes,
                    labelBuilder: (m) => _jogLabel(l10n, m),
                    onChanged: (m) => onChanged(
                      copyAppSettings(draft, defaultOuterJogMode: m),
                    ),
                  ),
                ),
              ),
            ],
          ),
        ),
        SettingsSectionCard(
          title: l10n.settingsDeckVinylBrakePause,
          subtitle: l10n.settingsDeckVinylBrakePauseHint,
          trailing: SettingsHeaderSwitch(
            value: draft.vinylBrakePause,
            semanticsLabel: l10n.settingsDeckVinylBrakePauseToggle,
            onChanged: (v) =>
                onChanged(copyAppSettings(draft, vinylBrakePause: v)),
          ),
          child: draft.vinylBrakePause
              ? SettingsField(
                  label: l10n.settingsDeckVinylBrakeStopTime,
                  hint: l10n.settingsDeckVinylBrakeStopTimeHint,
                  child: SettingsNumericStepper(
                    value: draft.vinylBrakeStopSecs,
                    min: kMinVinylBrakeStopSecs,
                    max: kMaxVinylBrakeStopSecs,
                    step: kVinylBrakeStopSecsStep,
                    format: (v) => '${v.toStringAsFixed(2)} s',
                    fieldLabel: l10n.settingsDeckVinylBrakeStopTime,
                    onChanged: (v) => onChanged(
                      copyAppSettings(draft, vinylBrakeStopSecs: v),
                    ),
                  ),
                )
              : null,
        ),
        SettingsSectionCard(
          title: l10n.settingsDeckTempoKeyTitle,
          subtitle: l10n.settingsDeckTempoKeyDescription,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            spacing: 16,
            children: [
              SettingsField(
                label: l10n.settingsDeckDefaultTempoRange,
                child: SettingsSelect(
                  dialogTitle: l10n.settingsDeckDefaultTempoRange,
                  value: draft.defaultTempoRange,
                  options: _tempoRangeOptions(draft),
                  labelBuilder: formatTempoRange,
                  onChanged: (step) => onChanged(
                    copyAppSettings(draft, defaultTempoRange: step),
                  ),
                ),
              ),
              SettingsField(
                label: l10n.settingsDeckDefaultKeyLock,
                hint: l10n.settingsDeckDefaultKeyLockHint,
                child: SettingsCheckbox(
                  label: l10n.settingsDeckKeyLock,
                  value: draft.defaultKeyLock,
                  onChanged: (v) =>
                      onChanged(copyAppSettings(draft, defaultKeyLock: v)),
                ),
              ),
            ],
          ),
        ),
        SettingsSectionCard(
          title: l10n.settingsDeckSamplerTitle,
          subtitle: l10n.settingsDeckSamplerDescription,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            spacing: 16,
            children: [
              SettingsField(
                label: l10n.settingsDeckSamplerPlayMode,
                child: SettingsSelect(
                  dialogTitle: l10n.settingsDeckSamplerPlayMode,
                  value: draft.samplerPlayMode,
                  options: SamplerPlayModeSetting.values,
                  labelBuilder: (m) => switch (m) {
                    SamplerPlayModeSetting.oneshot =>
                      l10n.settingsDeckSamplerPlayOneshot,
                    SamplerPlayModeSetting.hold =>
                      l10n.settingsDeckSamplerPlayHold,
                    SamplerPlayModeSetting.loop =>
                      l10n.settingsDeckSamplerPlayLoop,
                  },
                  onChanged: (m) =>
                      onChanged(copyAppSettings(draft, samplerPlayMode: m)),
                ),
              ),
              Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                spacing: 12,
                children: [
                  for (var deck = 0; deck < draft.numDecks; deck++)
                    Expanded(
                      child: SettingsField(
                        label: _deckBankLabel(l10n, deck),
                        child: SettingsSelect<String?>(
                          dialogTitle: _deckBankLabel(l10n, deck),
                          value: draft.deckDefaultSamplerBankId[deck],
                          options: _bankOptions(
                            banks,
                            draft.deckDefaultSamplerBankId[deck],
                          ),
                          labelBuilder: (id) => _bankLabel(l10n, banks, id),
                          onChanged: (bankId) => _setDeckBank(deck, bankId),
                        ),
                      ),
                    ),
                ],
              ),
            ],
          ),
        ),
      ],
    );
  }

  static String _deckBankLabel(AppLocalizations l10n, int deck) =>
      switch (deck) {
        0 => l10n.settingsDeckDefaultSamplerBankA,
        1 => l10n.settingsDeckDefaultSamplerBankB,
        2 => l10n.settingsDeckDefaultSamplerBankC,
        _ => l10n.settingsDeckDefaultSamplerBankD,
      };

  void _setDeckBank(int deck, String? bankId) {
    final banks = List<String?>.from(draft.deckDefaultSamplerBankId);
    while (banks.length < draft.numDecks) {
      banks.add(null);
    }
    banks[deck] = bankId;
    onChanged(copyAppSettings(draft, deckDefaultSamplerBankId: banks));
  }

  static List<double> _tempoRangeOptions(AppSettings draft) {
    const eps = 1e-4;
    final steps = [
      for (final step in draft.tempoRangeSteps)
        if (step.isFinite && step > 0) step,
    ];
    final options = steps.isEmpty ? List<double>.from(kTempoRangeSteps) : steps;
    if (!options.any((s) => (s - draft.defaultTempoRange).abs() < eps)) {
      options.insert(0, draft.defaultTempoRange);
    }
    return options;
  }

  static List<String?> _bankOptions(
    List<SamplerBankInfo> banks,
    String? selected,
  ) {
    return [
      null,
      if (selected != null && !banks.any((b) => b.id == selected)) selected,
      for (final bank in banks) bank.id,
    ];
  }

  static String _bankLabel(
    AppLocalizations l10n,
    List<SamplerBankInfo> banks,
    String? id,
  ) {
    if (id == null) {
      return l10n.commonNone;
    }
    for (final bank in banks) {
      if (bank.id == id) {
        return bank.name;
      }
    }
    return id;
  }

  static String _jogLabel(AppLocalizations l10n, JogModeSetting mode) =>
      switch (mode) {
        JogModeSetting.vinyl => l10n.settingsDeckJogVinyl,
        JogModeSetting.pitchBend => l10n.settingsDeckJogPitchBend,
        JogModeSetting.ignore => l10n.settingsDeckJogIgnore,
      };
}
