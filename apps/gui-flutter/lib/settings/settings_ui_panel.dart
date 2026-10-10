import 'package:flutter/widgets.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_field.dart';
import 'package:gui_flutter/settings/settings_widgets.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';

class SettingsUiPanel extends StatelessWidget {
  const new({required this.draft, required this.onChanged, super.key});

  final AppSettings draft;
  final ValueChanged<AppSettings> onChanged;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      spacing: 16,
      children: [
        const SettingsSectionHeader(
          title: 'UI',
          description: 'Chrome, hover tips, and control presentation.',
        ),
        SettingsPanel(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            spacing: 16,
            children: [
              SettingsToggle(
                label: 'Show tooltips',
                value: draft.showTooltips,
                onChanged: (enabled) =>
                    onChanged(copyAppSettings(draft, showTooltips: enabled)),
              ),
              SettingsField(
                label: 'Select style',
                child: SettingsSelect<SelectStyleSetting>(
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
