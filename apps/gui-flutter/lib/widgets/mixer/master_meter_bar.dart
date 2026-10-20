import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/widgets/mixer/level_meter.dart';
import 'package:gui_flutter/widgets/mixer/master_strip.dart';
import 'package:gui_flutter/widgets/shell/app_tooltip.dart';
import 'package:gui_flutter/widgets/shell/m_tappable.dart';
import 'package:gui_flutter/widgets/shell/mixar_dialog.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// Master stereo meter at the left of the FX bar; tap opens the Master +
/// Headphone dialog.
class MasterMeterBar extends ConsumerWidget {
  const new({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = context.theme;
    final levels = ref.watch(masterLevelsProvider);

    const tip = 'Master levels';
    return AppTooltip(
      tip: tip,
      child: MTappable(
        key: const ValueKey('master-meter'),
        semanticsLabel: tip,
        onPress: () {
          unawaited(
            showMixarDialog<void>(
              context: context,
              builder: (context) => const MasterStrip(),
            ),
          );
        },
        builder: (context, state) {
          // Plain button chrome: square, primary tint only while active.
          final wash = state.active ? theme.colors.primaryTint : null;
          return DecoratedBox(
            decoration: BoxDecoration(color: wash),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
              child: Row(
                spacing: 4,
                mainAxisSize: MainAxisSize.min,
                children: [
                  Text(
                    'MST',
                    style: theme.typography.body.xs.copyWith(
                      fontSize: 8,
                      fontWeight: FontWeight.w600,
                      color: theme.colors.mutedForeground,
                    ),
                  ),
                  SizedBox(
                    width: 72,
                    height: 13,
                    child: RepaintBoundary(
                      child: LevelMeter(
                        levels: levels,
                        mode: LevelMeterMode.stereo,
                        orientation: LevelMeterOrientation.horizontal,
                      ),
                    ),
                  ),
                ],
              ),
            ),
          );
        },
      ),
    );
  }
}
