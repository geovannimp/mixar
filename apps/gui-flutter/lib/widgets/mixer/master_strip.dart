import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/models/engine_ui.dart';
import 'package:gui_flutter/core/models/settings_section.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/widgets/mixer/fader_slider.dart';
import 'package:gui_flutter/widgets/mixer/mixer_button.dart';
import 'package:gui_flutter/widgets/settings/settings_field.dart';
import 'package:gui_flutter/widgets/settings/settings_insets.dart';
import 'package:gui_flutter/widgets/settings/settings_section_header.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/app_tooltip.dart';
import 'package:gui_flutter/widgets/shell/mixar_dialog.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:gui_flutter/widgets/shell/mixar_toast.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Master + headphone controls (dialog body).
///
/// Laid out as one settings-style card: a title band, then section headers and
/// fields separated by full-bleed rules. Keeping the whole dialog on the card
/// surface (instead of stacking separate cards) gives it one outer frame and
/// stops the sections reading as floating panels over the scrim.
class MasterStrip extends ConsumerWidget {
  const new({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = context.theme;
    final previewEnabled = ref
        .watch(appSettingsProvider)
        .maybeWhen(data: (s) => s.previewEnabled, orElse: () => false);
    final masterVolume = ref.watch(masterVolumeProvider) * 100;
    final cueVolume = ref.watch(cueVolumeProvider) * 100;
    final cueMix = ref.watch(cueMixProvider) * 100;
    final masterCue = ref.watch(masterCueProvider);
    final edgeLabelStyle = theme.typography.body.xs.copyWith(
      color: theme.colors.mutedForeground,
    );
    final readoutStyle = theme.typography.body.sm.copyWith(
      fontWeight: .w600,
      fontFeatures: const [FontFeature.tabularFigures()],
    );
    const headerPadding = EdgeInsets.symmetric(
      horizontal: settingsCardSideInset,
      vertical: 12,
    );
    const bodyPadding = EdgeInsets.symmetric(
      horizontal: settingsCardSideInset,
      vertical: 12,
    );

    Widget rule() => SizedBox(
      width: double.infinity,
      height: theme.style.borderWidth,
      child: ColoredBox(color: theme.colors.border),
    );

    return MixarDialog(
      title: 'Levels',
      contentPadding: EdgeInsets.zero,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        mainAxisSize: MainAxisSize.min,
        children: [
          const Padding(
            padding: headerPadding,
            child: SettingsSectionHeader(
              title: 'Master',
              description: 'Output bus volume and master cue.',
            ),
          ),
          rule(),
          Padding(
            padding: bodyPadding,
            child: SettingsField(
              label: 'Output level',
              trailing: Row(
                mainAxisSize: MainAxisSize.min,
                spacing: 8,
                children: [
                  Text('${masterVolume.round()}%', style: readoutStyle),
                  AppTooltip(
                    tip: 'Master cue',
                    child: MixerButton.icon(
                      variant: masterCue ? .secondary : .ghost,
                      size: .xs,
                      selected: masterCue,
                      semanticsLabel: 'Master cue',
                      onPress: () {
                        unawaited(
                          _masterCmd(
                            context,
                            () => setMasterCue(ref, !masterCue),
                          ),
                        );
                      },
                      child: Icon(
                        LucideIcons.headphones,
                        size: 14,
                        color: masterCue
                            ? theme.colors.primary
                            : theme.colors.mutedForeground,
                      ),
                    ),
                  ),
                ],
              ),
              child: SizedBox(
                height: 24,
                child: FaderSlider(
                  orientation: .horizontal,
                  value: masterVolume,
                  defaultValue: EngineUiSnapshot.empty.masterVolume * 100,
                  step: 0.05,
                  semanticLabel: 'Master level',
                  onValueChange: (next) {
                    unawaited(
                      _masterCmd(
                        context,
                        () => setMasterVolume(ref, next / 100),
                      ),
                    );
                  },
                ),
              ),
            ),
          ),
          const SizedBox(height: 16),
          rule(),
          Padding(
            padding: headerPadding,
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              spacing: 12,
              children: [
                const Expanded(
                  child: SettingsSectionHeader(
                    title: 'Headphone',
                    description: 'Cue volume and Cue/Master blend.',
                  ),
                ),
                if (!previewEnabled)
                  AppButton(
                    variant: .outline,
                    size: .sm,
                    mainAxisSize: .min,
                    semanticsLabel: 'Enable preview bus in settings',
                    onPress: () {
                      ref
                          .read(pendingSettingsDeepLinkProvider.notifier)
                          .request(
                            const SettingsDeepLink(
                              section: SettingsSection.audio,
                              scrollToPreviewBus: true,
                            ),
                          );
                      Navigator.of(context).pop();
                    },
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      spacing: 6,
                      children: [
                        const Text('Enable in settings'),
                        Icon(
                          LucideIcons.chevronRight,
                          size: 14,
                          color: theme.colors.foreground,
                        ),
                      ],
                    ),
                  ),
              ],
            ),
          ),
          rule(),
          Padding(
            padding: bodyPadding,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              mainAxisSize: MainAxisSize.min,
              spacing: 16,
              children: [
                SettingsField(
                  label: 'Cue level',
                  trailing: Text('${cueVolume.round()}%', style: readoutStyle),
                  child: SizedBox(
                    height: 24,
                    child: FaderSlider(
                      orientation: .horizontal,
                      value: cueVolume,
                      defaultValue: EngineUiSnapshot.empty.cueVolume * 100,
                      step: 0.05,
                      semanticLabel: 'Headphone level',
                      onValueChange: (next) {
                        unawaited(
                          _masterCmd(
                            context,
                            () => setCueVolume(ref, next / 100),
                          ),
                        );
                      },
                    ),
                  ),
                ),
                SettingsField(
                  label: 'Mixing',
                  child: SizedBox(
                    height: 32,
                    child: Row(
                      spacing: 12,
                      children: [
                        Text('Cue', style: edgeLabelStyle),
                        Expanded(
                          child: FaderSlider(
                            orientation: .horizontal,
                            value: cueMix,
                            defaultValue: EngineUiSnapshot.empty.cueMix * 100,
                            step: 0.05,
                            showIndicator: false,
                            showMarkers: true,
                            centerNotch: true,
                            semanticLabel: 'Cue/Mst',
                            onValueChange: (next) {
                              unawaited(
                                _masterCmd(
                                  context,
                                  () => setCueMix(ref, next / 100),
                                ),
                              );
                            },
                          ),
                        ),
                        Text('Mst', style: edgeLabelStyle),
                      ],
                    ),
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

Future<void> _masterCmd(
  BuildContext context,
  Future<void> Function() fn,
) async {
  try {
    await fn();
  } catch (e) {
    if (!context.mounted) {
      return;
    }
    showMixarToast(
      context: context,
      variant: MixarToastVariant.destructive,
      title: Text('$e'),
    );
  }
}
