import 'dart:async';
import 'dart:math' as math;

import 'package:anchor_ui/anchor_ui.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/utils/fx_params.dart';
import 'package:gui_flutter/core/utils/mixer_live_color.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/src/rust/api/engine.dart' show FxKind;
import 'package:gui_flutter/widgets/mixer/master_meter_bar.dart';
import 'package:gui_flutter/widgets/mixer/mixer_button.dart';
import 'package:gui_flutter/widgets/shell/app_tooltip.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_anchor.dart';
import 'package:gui_flutter/widgets/shell/mixar_slider.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:gui_flutter/widgets/shell/mixar_toast.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Status-bar-sized Beat FX strip under the library (`mixer_page.dart`).
///
/// Mirrors the DDJ-400 panel in one line: effect select, BEAT ◀ ▶, LEVEL/DEPTH
/// and ON/OFF. The unit is shared, so both decks run the same effect; each
/// deck's header `FX` button decides whether that deck's signal feeds it.
class FxBar extends ConsumerWidget {
  const new({super.key});

  /// Fixed bar height (`mixer_page.dart` reserves it).
  static const double height = 28;

  /// Width of the LEVEL slider inside the strip.
  static const double _levelWidth = 110;

  /// Horizontal padding of the strip.
  static const double _pad = 8;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = context.theme;
    final slots = ref.watch(fxSlotsProvider);
    final state = slots.length > kBeatFxSlot ? slots[kBeatFxSlot] : null;
    final running = ref.watch(engineRunningProvider);
    // The unit always holds an effect (the select has no "off" entry).
    final kind = state?.kind ?? FxKind.echo;
    final on = state?.enabled ?? false;
    final wet = state?.wet ?? 0.5;
    final divisions = fxDivisionLabels(kind);
    // LEVEL is settable while the unit is off, exactly like the hardware.
    final selected = running && kind != FxKind.off;
    final controls = <Widget>[
      _EffectMenu(
        kind: kind,
        enabled: running,
        onSelected: (next) {
          unawaited(
            _fxCmd(context, () => setFxUnitSlot(ref, kBeatFxSlot, next)),
          );
        },
      ),
      // Hidden for effects with no beat time (reverb).
      if (divisions.isNotEmpty)
        _BeatStepper(
          divisions: divisions,
          index: fxDivisionIndexFrom(state?.params),
          enabled: selected,
          onStep: (index) {
            unawaited(
              _fxCmd(context, () => setFxUnitDivision(ref, kBeatFxSlot, index)),
            );
          },
        ),
      const _Caption('LEVEL'),
      AppTooltip(
        tip: 'Level / depth ${(wet * 100).round()}%',
        description: 'Wet/dry mix of the shared FX unit.',
        child: SizedBox(
          width: _levelWidth,
          height: 16,
          child: Center(
            child: MixarSlider(
              value: wet,
              enabled: selected,
              trackHeight: 4,
              thumbRadius: 6,
              onChanged: (value) {
                unawaited(
                  _fxCmd(context, () => setFxUnitWet(ref, kBeatFxSlot, value)),
                );
              },
            ),
          ),
        ),
      ),
    ];
    return ColoredBox(
      color: theme.colors.card,
      child: Row(
        children: [
          // Master meter + dialog opener, pinned to the bar's left edge.
          const Padding(
            padding: EdgeInsets.only(left: _pad),
            child: MasterMeterBar(),
          ),
          Expanded(
            child: LayoutBuilder(
              builder: (context, constraints) => SingleChildScrollView(
                scrollDirection: Axis.horizontal,
                padding: const EdgeInsets.symmetric(horizontal: _pad),
                child: ConstrainedBox(
                  constraints: BoxConstraints(
                    minWidth: constraints.maxWidth.isFinite
                        ? math.max<double>(0, constraints.maxWidth - _pad * 2)
                        : 0,
                  ),
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    mainAxisAlignment: MainAxisAlignment.end,
                    spacing: 8,
                    children: [
                      Row(
                        mainAxisSize: MainAxisSize.min,
                        spacing: 8,
                        children: controls,
                      ),
                      _ArmButton(
                        on: on,
                        enabled: selected,
                        onPress: () {
                          unawaited(
                            _fxCmd(
                              context,
                              () => setFxUnitSlotEnabled(ref, kBeatFxSlot, !on),
                            ),
                          );
                        },
                      ),
                    ],
                  ),
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

/// Status-strip caption.
class _Caption extends StatelessWidget {
  const new(this.text);

  final String text;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return Text(
      text,
      style: theme.typography.body.xs2.copyWith(
        fontWeight: .w600,
        letterSpacing: 1.4,
        color: theme.colors.mutedForeground,
      ),
    );
  }
}

/// FX SELECT: the effect name opens a menu of the available effects.
class _EffectMenu extends StatelessWidget {
  const new({
    required this.kind,
    required this.enabled,
    required this.onSelected,
  });

  final FxKind kind;
  final bool enabled;
  final ValueChanged<FxKind> onSelected;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final label = theme.typography.body.xs.copyWith(
      fontWeight: .w600,
      color: theme.colors.foreground,
    );
    return MixarMenuAnchor(
      placement: Placement.topStart,
      dialogTitle: 'FX select',
      minWidth: 160,
      enabled: enabled,
      menuBuilder: (context, controller) => Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        mainAxisSize: MainAxisSize.min,
        children: [
          for (final option in const [FxKind.echo, FxKind.reverb])
            MixerButton(
              variant: .ghost,
              size: .xs,
              selected: option == kind,
              semanticsLabel: fxKindLabel(option),
              onPress: () {
                controller.hide();
                onSelected(option);
              },
              child: Text(fxKindLabel(option)),
            ),
        ],
      ),
      childBuilder: (context, controller) => MixerButton(
        variant: .ghost,
        size: .xs,
        mainAxisSize: .min,
        semanticsLabel: 'FX select ${fxKindLabel(kind)}',
        onPress: enabled ? controller.toggle : null,
        child: Row(
          mainAxisSize: MainAxisSize.min,
          spacing: 4,
          children: [
            Text(fxKindLabel(kind), style: label),
            Icon(
              LucideIcons.chevronDown,
              size: 12,
              color: theme.colors.mutedForeground,
            ),
          ],
        ),
      ),
    );
  }
}

/// BEAT ◀ ▶: steps the selected effect's beat division.
class _BeatStepper extends StatelessWidget {
  const new({
    required this.divisions,
    required this.index,
    required this.enabled,
    required this.onStep,
  });

  final List<String> divisions;
  final int index;
  final bool enabled;
  final ValueChanged<int> onStep;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return Row(
      mainAxisSize: MainAxisSize.min,
      spacing: 2,
      children: [
        MixerButton.icon(
          variant: .ghost,
          size: .xs,
          semanticsLabel: 'Beat division down',
          onPress: enabled && index > 0 ? () => onStep(index - 1) : null,
          child: const Icon(LucideIcons.chevronLeft, size: 12),
        ),
        SizedBox(
          width: 32,
          child: Text(
            divisions[index],
            textAlign: TextAlign.center,
            style: theme.typography.body.xs.copyWith(
              fontWeight: .w600,
              color: enabled
                  ? theme.colors.foreground
                  : theme.colors.mutedForeground,
            ),
          ),
        ),
        MixerButton.icon(
          variant: .ghost,
          size: .xs,
          semanticsLabel: 'Beat division up',
          onPress: enabled && index < divisions.length - 1
              ? () => onStep(index + 1)
              : null,
          child: const Icon(LucideIcons.chevronRight, size: 12),
        ),
      ],
    );
  }
}

/// ON/OFF: the `BEAT FX` label itself, lit while the unit is armed.
class _ArmButton extends StatelessWidget {
  const new({required this.on, required this.enabled, required this.onPress});

  final bool on;
  final bool enabled;
  final VoidCallback onPress;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final lit = on && enabled;
    return AppTooltip(
      tip: on ? 'Beat FX on' : 'Beat FX off',
      description: 'Arm the shared FX unit.',
      child: MixerButton(
        variant: .ghost,
        size: .xs,
        mainAxisSize: .min,
        selected: lit,
        semanticsLabel: 'FX ${on ? 'on' : 'off'}',
        backgroundColor: lit ? theme.colors.primaryTint : null,
        onPress: enabled ? onPress : null,
        child: Text(
          'BEAT FX',
          style: theme.typography.body.xs2.copyWith(
            fontWeight: .w600,
            letterSpacing: 1.4,
            color: lit ? kMixerLiveOn : theme.colors.mutedForeground,
          ),
        ),
      ),
    );
  }
}

Future<void> _fxCmd(BuildContext context, Future<void> Function() fn) async {
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
