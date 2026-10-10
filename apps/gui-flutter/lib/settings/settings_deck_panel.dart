import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/mixer/tempo_format.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_field.dart';
import 'package:gui_flutter/settings/settings_providers.dart';
import 'package:gui_flutter/settings/settings_widgets.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';

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
        SettingsPanel(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            spacing: 16,
            children: [
              SettingsSectionHeader(
                title: l10n.settingsDeckJogTitle,
                description: l10n.settingsDeckJogDescription,
              ),
              const SizedBox(height: 0),
              Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                spacing: 12,
                children: [
                  Expanded(
                    child: SettingsField(
                      label: l10n.settingsDeckTopJogMode,
                      child: SettingsSelect(
                        dialogTitle: 'Top jog mode',
                        value: draft.defaultTopJogMode,
                        options: _jogModes,
                        labelBuilder: (m) => _jogLabel(l10n, m),
                        onChanged: (m) => onChanged(
                          copyAppSettings(draft, defaultTopJogMode: m),
                        ),
                      ),
                    ),
                  ),
                  Expanded(
                    child: SettingsField(
                      label: l10n.settingsDeckOuterJogMode,
                      child: SettingsSelect(
                        dialogTitle: 'Outer jog mode',
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
            ],
          ),
        ),
        SettingsPanel(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            spacing: 16,
            children: [
              SettingsSectionHeader(
                title: l10n.settingsDeckTempoKeyTitle,
                description: l10n.settingsDeckTempoKeyDescription,
              ),
              const SizedBox(height: 0),
              SettingsField(
                label: l10n.settingsDeckDefaultTempoRange,
                child: SettingsSelect(
                  dialogTitle: 'Default tempo range',
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
                child: SettingsToggle(
                  label: l10n.settingsDeckKeyLock,
                  value: draft.defaultKeyLock,
                  onChanged: (v) =>
                      onChanged(copyAppSettings(draft, defaultKeyLock: v)),
                ),
              ),
            ],
          ),
        ),
        SettingsPanel(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            spacing: 16,
            children: [
              SettingsSectionHeader(
                title: l10n.settingsDeckSamplerTitle,
                description: l10n.settingsDeckSamplerDescription,
              ),
              const SizedBox(height: 0),
              SettingsField(
                label: l10n.settingsDeckSamplerPlayMode,
                child: SettingsSelect(
                  dialogTitle: 'Sampler play mode',
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
              SettingsField(
                label: l10n.settingsDeckSamplerStripRoute,
                child: SettingsSelect(
                  dialogTitle: 'Sampler strip route',
                  value: draft.samplerStripRoute,
                  options: SamplerStripRouteSettingFrb.values,
                  labelBuilder: (m) => m == SamplerStripRouteSettingFrb.before
                      ? l10n.settingsDeckSamplerStripBefore
                      : l10n.settingsDeckSamplerStripAfter,
                  onChanged: (m) =>
                      onChanged(copyAppSettings(draft, samplerStripRoute: m)),
                ),
              ),
              Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                spacing: 12,
                children: [
                  for (var deck = 0; deck < 2; deck++)
                    Expanded(
                      child: SettingsField(
                        label: deck == 0
                            ? l10n.settingsDeckDefaultSamplerBankA
                            : l10n.settingsDeckDefaultSamplerBankB,
                        child: SettingsSelect<String?>(
                          dialogTitle:
                              'Deck ${deck == 0 ? 'A' : 'B'} default sampler bank',
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

  void _setDeckBank(int deck, String? bankId) {
    final banks = List<String?>.from(draft.deckDefaultSamplerBankId);
    while (banks.length < 2) {
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
