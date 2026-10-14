import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/mixer/level_meter.dart';
import 'package:gui_flutter/mixer/master_strip.dart';
import 'package:gui_flutter/shell/app_tooltip.dart';
import 'package:gui_flutter/shell/m_tappable.dart';
import 'package:gui_flutter/shell/mixar_dialog.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';

/// Header master stereo meter; tap opens Master + Headphone dialog.
class MasterMeterHeader extends ConsumerWidget {
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
          final wash = state.active
              ? theme.colors.hover(theme.colors.secondary)
              : null;
          return DecoratedBox(
            decoration: BoxDecoration(
              color: wash,
              borderRadius: BorderRadius.circular(4),
            ),
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
