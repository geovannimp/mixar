import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/mixer/tempo_format.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_field.dart';
import 'package:gui_flutter/settings/settings_providers.dart';
import 'package:gui_flutter/settings/settings_widgets.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';

class SettingsDeckPanel extends ConsumerWidget {
  const SettingsDeckPanel({
    super.key,
    required this.draft,
    required this.onChanged,
  });

  final AppSettings draft;
  final ValueChanged<AppSettings> onChanged;

  static const _jogModes = JogModeSetting.values;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final banks = ref.watch(samplerBanksProvider).value ?? const [];

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      spacing: 16,
      children: [
        const SettingsSectionHeader(
          title: 'Deck',
          description:
              'Default jog, tempo, and sampler behavior for new decks.',
        ),
        SettingsPanel(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            spacing: 16,
            children: [
              const SettingsSectionHeader(
                title: 'Jog wheel',
                description:
                    'Defaults for top (touch) and outer (freewheel) platter policy.',
              ),
              const SizedBox(height: 0),
              Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                spacing: 12,
                children: [
                  Expanded(
                    child: SettingsField(
                      label: 'Top jog mode',
                      child: SettingsSelect(
                        value: draft.defaultTopJogMode,
                        options: _jogModes,
                        labelBuilder: _jogLabel,
                        onChanged: (m) => onChanged(
                          copyAppSettings(draft, defaultTopJogMode: m),
                        ),
                      ),
                    ),
                  ),
                  Expanded(
                    child: SettingsField(
                      label: 'Outer jog mode',
                      child: SettingsSelect(
                        value: draft.defaultOuterJogMode,
                        options: _jogModes,
                        labelBuilder: _jogLabel,
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
              const SettingsSectionHeader(
                title: 'Tempo and Key',
                description:
                    'Default pitch-fader range and key lock for new decks.',
              ),
              const SizedBox(height: 0),
              SettingsField(
                label: 'Default tempo range',
                child: SettingsSelect(
                  value: draft.defaultTempoRange,
                  options: _tempoRangeOptions(draft),
                  labelBuilder: formatTempoRange,
                  onChanged: (step) => onChanged(
                    copyAppSettings(draft, defaultTempoRange: step),
                  ),
                ),
              ),
              SettingsField(
                label: 'Default key lock',
                hint:
                    'Tempo-only pitch when on (time-stretch). Off = vinyl tempo.',
                child: SettingsToggle(
                  label: 'Key lock',
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
              const SettingsSectionHeader(
                title: 'Sampler',
                description:
                    'Default play mode for inherit banks and default bank per deck.',
              ),
              const SizedBox(height: 0),
              SettingsField(
                label: 'Sampler play mode',
                child: SettingsSelect(
                  value: draft.samplerPlayMode,
                  options: SamplerPlayModeSetting.values,
                  labelBuilder: (m) => m.name,
                  onChanged: (m) =>
                      onChanged(copyAppSettings(draft, samplerPlayMode: m)),
                ),
              ),
              SettingsField(
                label: 'Sampler strip route',
                child: SettingsSelect(
                  value: draft.samplerStripRoute,
                  options: SamplerStripRouteSettingFrb.values,
                  labelBuilder: (m) => m == SamplerStripRouteSettingFrb.before
                      ? 'Before channel strip'
                      : 'After channel strip',
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
                        label:
                            'Deck ${deck == 0 ? 'A' : 'B'} default sampler bank',
                        child: SettingsSelect<String?>(
                          value: draft.deckDefaultSamplerBankId[deck],
                          options: _bankOptions(
                            banks,
                            draft.deckDefaultSamplerBankId[deck],
                          ),
                          labelBuilder: (id) => _bankLabel(banks, id),
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

  static String _bankLabel(List<SamplerBankInfo> banks, String? id) {
    if (id == null) {
      return 'None';
    }
    for (final bank in banks) {
      if (bank.id == id) {
        return bank.name;
      }
    }
    return id;
  }

  static String _jogLabel(JogModeSetting mode) => switch (mode) {
    JogModeSetting.vinyl => 'Vinyl (scratch)',
    JogModeSetting.pitchBend => 'Pitch bend',
    JogModeSetting.ignore => 'Ignore',
  };
}
