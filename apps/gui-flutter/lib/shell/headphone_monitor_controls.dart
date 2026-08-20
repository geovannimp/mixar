import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/mixer/rotary_knob.dart';
import 'package:gui_flutter/settings/settings_providers.dart';

const _cueOn = Color(0xFFFCD34D); // amber-300
const _cueRing = Color(0x66F59E0B); // amber-500/40

/// Compact Master Cue + Cue/Master mix for the app header (Tauri parity).
class HeadphoneMonitorControls extends ConsumerWidget {
  const HeadphoneMonitorControls({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final previewEnabled = ref
        .watch(appSettingsProvider)
        .maybeWhen(data: (s) => s.previewEnabled, orElse: () => false);
    final cueMix = ref.watch(cueMixProvider);
    final masterCue = ref.watch(masterCueProvider);
    final theme = context.theme;

    return Row(
      mainAxisSize: .min,
      spacing: 8,
      children: [
        FButton(
          variant: masterCue ? .secondary : .outline,
          size: .sm,
          mainAxisSize: .min,
          onPress: previewEnabled
              ? () {
                  unawaited(
                    _headerCmd(context, () => setMasterCue(ref, !masterCue)),
                  );
                }
              : null,
          child: Text(
            'Master Cue',
            style: theme.typography.body.xs.copyWith(
              fontWeight: .w600,
              letterSpacing: 0.6,
              fontSize: 9,
              color: masterCue ? _cueOn : theme.colors.mutedForeground,
            ),
          ),
        ),
        RotaryKnob(
          label: 'Cue/Mst',
          value: cueMix,
          min: 0,
          max: 1,
          step: 0.01,
          size: .sm,
          disabled: !previewEnabled,
          accentColor: theme.colors.mutedForeground,
          ringColor: _cueRing,
          onValueChange: (mix) {
            unawaited(_headerCmd(context, () => setCueMix(ref, mix)));
          },
        ),
      ],
    );
  }
}

Future<void> _headerCmd(
  BuildContext context,
  Future<void> Function() fn,
) async {
  try {
    await fn();
  } catch (e) {
    if (!context.mounted) {
      return;
    }
    showFToast(context: context, variant: .destructive, title: Text('$e'));
  }
}
