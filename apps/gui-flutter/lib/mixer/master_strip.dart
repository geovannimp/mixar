import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/mixer/fader_slider.dart';
import 'package:gui_flutter/mixer/mixer_button.dart';
import 'package:gui_flutter/mixer/mixer_live_color.dart';
import 'package:gui_flutter/settings/settings_providers.dart';
import 'package:gui_flutter/settings/settings_section.dart';
import 'package:gui_flutter/settings/settings_widgets.dart';
import 'package:gui_flutter/shell/app_button.dart';
import 'package:gui_flutter/shell/app_tooltip.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:gui_flutter/shell/mixar_toast.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Master + headphone controls (dialog body).
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
    final labelStyle = theme.typography.body.xs.copyWith(
      color: theme.colors.mutedForeground,
      fontWeight: .w600,
      fontSize: 8,
    );

    return Padding(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        mainAxisSize: MainAxisSize.min,
        spacing: 16,
        children: [
          Text(
            'Levels',
            style: theme.typography.body.md.copyWith(fontWeight: .w700),
          ),
          SettingsSectionCard(
            title: 'Master',
            subtitle: 'Output bus volume and master cue.',
            child: SizedBox(
              height: 24,
              child: Row(
                spacing: 8,
                children: [
                  Expanded(
                    child: FaderSlider(
                      orientation: .horizontal,
                      value: masterVolume,
                      step: 0.05,
                      showMarkers: true,
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
                  Center(
                    child: AppTooltip(
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
                              ? kMixerLiveOn
                              : theme.colors.mutedForeground,
                        ),
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ),
          SettingsSectionCard(
            title: 'Headphone',
            subtitle: 'Cue volume and Cue/Master blend.',
            trailing: previewEnabled
                ? null
                : AppButton(
                    variant: .outline,
                    size: .xs,
                    mainAxisSize: .min,
                    semanticsLabel: 'Enable preview bus',
                    onPress: () {
                      Navigator.of(context).pop();
                      ref
                          .read(pendingSettingsDeepLinkProvider.notifier)
                          .request(
                            const SettingsDeepLink(
                              section: SettingsSection.audio,
                              scrollToPreviewBus: true,
                            ),
                          );
                    },
                    child: const Text('Enable'),
                  ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              mainAxisSize: MainAxisSize.min,
              children: [
                SizedBox(
                  height: 24,
                  child: FaderSlider(
                    orientation: .horizontal,
                    value: cueVolume,
                    step: 0.05,
                    showMarkers: true,
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
                const SizedBox(height: 8),
                Text('Mixing', textAlign: TextAlign.center, style: labelStyle),
                const SizedBox(height: 4),
                SizedBox(
                  height: 24,
                  child: Row(
                    spacing: 8,
                    children: [
                      Text('Cue', style: labelStyle),
                      Expanded(
                        child: FaderSlider(
                          orientation: .horizontal,
                          value: cueMix,
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
                      Text('Mst', style: labelStyle),
                    ],
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
