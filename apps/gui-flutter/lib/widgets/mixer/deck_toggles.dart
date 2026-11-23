import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/utils/mixer_live_color.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/widgets/mixer/mixer_button.dart';
import 'package:gui_flutter/widgets/mixer/mixer_command.dart';
import 'package:gui_flutter/widgets/shell/app_tooltip.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Master, quantize, slip, stem EQ, and FX for one deck.
///
/// The cluster lives in the deck's track-info card, so the deck keeps a single
/// chrome row; the key-lock chip sits beside it.
class DeckToggles extends ConsumerWidget {
  const new({required this.deckId, super.key});

  final int deckId;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final engineRunning = ref.watch(engineRunningProvider);
    final loadDisabled =
        ref.watch(deckLoadingProvider(deckId)) || !engineRunning;
    final isMaster = ref.watch(deckIsMasterProvider(deckId));
    final quantize = ref.watch(deckQuantizeProvider(deckId));
    final slip = ref.watch(deckSlipEnabledProvider(deckId));
    final stemsAudible = ref.watch(deckStemsAudibleProvider(deckId));
    final stemEqEnabled = ref.watch(deckStemEqEnabledProvider(deckId));
    final fxRouted = ref.watch(deckFxRoutedProvider(deckId));

    return Row(
      mainAxisSize: MainAxisSize.min,
      spacing: 4,
      children: [
        _MasterButton(
          isMaster: isMaster,
          disabled: loadDisabled,
          onPress: () {
            unawaited(
              runMixerCommand(context, () => setMasterDeck(ref, deckId)),
            );
          },
        ),
        _QuantizeButton(
          quantize: quantize,
          disabled: loadDisabled,
          onPress: () {
            // Capture the desired state before the async command.
            final enabled = !quantize;
            unawaited(
              runMixerCommand(
                context,
                () => setDeckQuantize(ref, deckId, enabled),
              ),
            );
          },
        ),
        _SlipButton(
          slip: slip,
          disabled: loadDisabled,
          onPress: () {
            final enabled = !slip;
            unawaited(
              runMixerCommand(context, () => setDeckSlip(ref, deckId, enabled)),
            );
          },
        ),
        _StemEqButton(
          stemsAudible: stemsAudible,
          stemEqOn: stemEqEnabled,
          disabled: !engineRunning,
          onPress: () {
            final enabled = !stemEqEnabled;
            unawaited(
              runMixerCommand(
                context,
                () => setDeckStemEqEnabled(ref, deckId, enabled),
              ),
            );
          },
        ),
        _FxButton(
          routed: fxRouted,
          disabled: !engineRunning,
          onPress: () {
            final routed = !fxRouted;
            unawaited(
              runMixerCommand(
                context,
                () => setDeckFxRouted(ref, deckId, routed),
              ),
            );
          },
        ),
      ],
    );
  }
}

/// Shared chrome for the five deck toggles. Callers supply only the
/// state-dependent look and press handler.
Widget _deckHeaderChip({
  required BuildContext context,
  required bool selected,
  required VoidCallback? onPress,
  required String semanticsLabel,
  required Widget child,
  Color? backgroundColor,
}) {
  return MixerButton(
    variant: .outline,
    size: .xs,
    mainAxisSize: .min,
    selected: selected,
    borderRadius: context.theme.style.borderRadius.pill,
    onPress: onPress,
    semanticsLabel: semanticsLabel,
    padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 2),
    backgroundColor: backgroundColor,
    child: child,
  );
}

class _MasterButton extends StatelessWidget {
  const new({
    required this.onPress,
    required this.isMaster,
    this.disabled = false,
  });

  final VoidCallback onPress;
  final bool isMaster;
  final bool disabled;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final on = isMaster && !disabled;
    final tip = isMaster ? 'Master' : 'Set master';
    return AppTooltip(
      tip: tip,
      description: isMaster
          ? 'This deck is the sync reference.'
          : 'Make this deck the tempo reference for sync.',
      child: _deckHeaderChip(
        context: context,
        selected: on,
        onPress: (!disabled && !isMaster) ? onPress : null,
        semanticsLabel: tip,
        backgroundColor: on ? kMixerMasterFill : null,
        child: Icon(
          LucideIcons.crown,
          size: 14,
          color: on ? kMixerMasterOn : theme.colors.mutedForeground,
        ),
      ),
    );
  }
}

class _QuantizeButton extends StatelessWidget {
  const new({
    required this.onPress,
    required this.quantize,
    this.disabled = false,
  });

  final VoidCallback onPress;
  final bool quantize;
  final bool disabled;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final on = quantize && !disabled;
    final tip = quantize ? 'Quantize on' : 'Quantize off';
    return AppTooltip(
      tip: tip,
      description:
          'Snap cues, loops, and hot cues to the grid, and start in time with '
          'the playing deck.',
      child: _deckHeaderChip(
        context: context,
        selected: on,
        onPress: disabled ? null : onPress,
        semanticsLabel: tip,
        backgroundColor: on ? theme.colors.primaryTint : null,
        child: Text(
          'Q',
          style: TextStyle(
            fontWeight: .w600,
            color: on ? theme.colors.primary : theme.colors.mutedForeground,
          ),
        ),
      ),
    );
  }
}

class _SlipButton extends StatelessWidget {
  const new({required this.onPress, required this.slip, this.disabled = false});

  final VoidCallback onPress;
  final bool slip;
  final bool disabled;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    const slipOn = Color.fromARGB(191, 96, 165, 250);
    const slipFill = Color.fromARGB(28, 96, 165, 250);
    final on = slip && !disabled;
    final tip = slip ? 'Slip on' : 'Slip off';
    return AppTooltip(
      tip: tip,
      description:
          'Shadow playhead keeps moving during loops and scratch; catch up on '
          'exit.',
      child: _deckHeaderChip(
        context: context,
        selected: on,
        onPress: disabled ? null : onPress,
        semanticsLabel: tip,
        backgroundColor: on ? slipFill : null,
        child: Text(
          'S',
          style: TextStyle(
            fontWeight: .w600,
            color: on ? slipOn : theme.colors.mutedForeground,
          ),
        ),
      ),
    );
  }
}

class _StemEqButton extends StatelessWidget {
  const new({
    required this.onPress,
    required this.stemsAudible,
    this.stemEqOn = false,
    this.disabled = false,
  });

  final VoidCallback onPress;

  /// The deck is really playing stems, so routing the knobs is meaningful.
  final bool stemsAudible;

  /// Current stem-EQ routing; lit while the EQ knobs drive the stems.
  final bool stemEqOn;
  final bool disabled;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    // Lit only while it is real: stems can drop out underneath a stored "on".
    final on = stemEqOn && stemsAudible && !disabled;
    final tip = !stemsAudible
        ? 'Stem EQ unavailable'
        : stemEqOn
        ? 'Stem EQ on'
        : 'Stem EQ off';
    return AppTooltip(
      tip: tip,
      description: stemsAudible
          ? 'Route the mixer EQ knobs to the stems instead of the channel: '
                'VOC, B/O, DRM.'
          : 'The deck is not playing stems right now, so the knobs stay on the '
                'channel EQ.',
      child: _deckHeaderChip(
        context: context,
        selected: on,
        onPress: disabled || !stemsAudible ? null : onPress,
        semanticsLabel: tip,
        backgroundColor: on ? theme.colors.primaryTint : null,
        child: Text(
          'ST',
          style: TextStyle(
            fontWeight: .w600,
            color: on ? theme.colors.primary : theme.colors.mutedForeground,
          ),
        ),
      ),
    );
  }
}

class _FxButton extends StatelessWidget {
  const new({
    required this.onPress,
    required this.routed,
    this.disabled = false,
  });

  final VoidCallback onPress;
  final bool routed;
  final bool disabled;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final on = routed && !disabled;
    final tip = routed ? 'FX on' : 'FX off';
    return AppTooltip(
      tip: tip,
      description: 'Route this deck through the FX unit.',
      child: _deckHeaderChip(
        context: context,
        selected: on,
        onPress: disabled ? null : onPress,
        semanticsLabel: tip,
        backgroundColor: on ? theme.colors.primaryTint : null,
        child: Text(
          'FX',
          style: TextStyle(
            fontWeight: .w600,
            color: on ? theme.colors.primary : theme.colors.mutedForeground,
          ),
        ),
      ),
    );
  }
}
