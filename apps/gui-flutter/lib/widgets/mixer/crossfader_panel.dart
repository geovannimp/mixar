import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/models/bus_enums.dart';
import 'package:gui_flutter/core/models/engine_ui.dart';
import 'package:gui_flutter/core/theme/deck_accent.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/widgets/mixer/fader_slider.dart';
import 'package:gui_flutter/widgets/mixer/mixer_command.dart';
import 'package:gui_flutter/widgets/shell/m_tappable.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_anchor.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_item.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Mixer foot: the section rule and the crossfader on their own panel.
///
/// Shared with the waveform center column, which keeps the crossfader under the
/// lanes instead of hiding it with the rest of the mixer.
class CrossfaderPanel extends StatelessWidget {
  const new({this.edgeFill = 0, super.key});

  /// Extra width each endpoint cell takes on its outer side, so the A/B chips
  /// fill the deck gutter while the fader keeps its width.
  final double edgeFill;

  @override
  Widget build(BuildContext context) {
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        const _SectionRule(),
        _CrossfaderStrip(edgeFill: edgeFill),
      ],
    );
  }
}

/// Section hairline spanning the whole mixer foot, deck card to deck card.
class _SectionRule extends StatelessWidget {
  const new();

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return SizedBox(
      height: theme.style.borderWidth,
      width: double.infinity,
      child: ColoredBox(
        key: const ValueKey('crossfader-rule'),
        color: theme.colors.border,
      ),
    );
  }
}

/// Base width of a crossfader assignment cell.
///
/// Each endpoint adds the gutter on its outer side ([CrossfaderPanel.edgeFill]),
/// and [CrossAxisAlignment.stretch] makes both cells fill the foot height.
const _crossfaderAssignExtent = 32.0;

/// Height of the crossfader foot below the section rule.
///
/// The old chrome (spacing + vertical padding + foot inset) added up to the
/// same band; now the A/B cells fill every pixel of it, border to border.
const _crossfaderFootHeight = 64.0;

class _CrossfaderStrip extends ConsumerWidget {
  const new({required this.edgeFill});

  /// Extra width the outer side of each endpoint cell takes from the gutter.
  final double edgeFill;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final enabled = ref.watch(engineRunningProvider);
    final crossfader = ref.watch(crossfaderProvider) * 100;
    return SizedBox(
      height: _crossfaderFootHeight,
      child: Row(
        spacing: 4,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _CrossfaderAssignmentMenu(
            key: const ValueKey('crossfader-assign-a'),
            side: CrossfaderAssign.a,
            width: _crossfaderAssignExtent + edgeFill,
          ),
          Expanded(
            child: Center(
              child: SizedBox(
                height: _crossfaderAssignExtent,
                width: double.infinity,
                child: FaderSlider(
                  orientation: .horizontal,
                  value: crossfader,
                  defaultValue: EngineUiSnapshot.empty.crossfader * 100,
                  step: 0.05,
                  showIndicator: false,
                  showMarkers: true,
                  centerNotch: true,
                  crossfaderTrack: true,
                  disabled: !enabled,
                  onValueChange: (next) {
                    unawaited(
                      runMixerCommand(
                        context,
                        () => setCrossfader(ref, next / 100),
                      ),
                    );
                  },
                ),
              ),
            ),
          ),
          _CrossfaderAssignmentMenu(
            key: const ValueKey('crossfader-assign-b'),
            side: CrossfaderAssign.b,
            width: _crossfaderAssignExtent + edgeFill,
          ),
        ],
      ),
    );
  }
}

class _CrossfaderAssignmentMenu extends ConsumerWidget {
  const new({required this.side, required this.width, super.key});

  final CrossfaderAssign side;
  final double width;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final enabled = ref.watch(engineRunningProvider);
    final theme = context.theme;
    final label = side == CrossfaderAssign.a ? 'A' : 'B';
    final color = deckInk(
      side == CrossfaderAssign.a ? DeckAccent.a : DeckAccent.b,
      theme.colors.brightness,
    );
    return MixarMenuAnchor(
      enabled: enabled,
      minWidth: 160,
      dialogTitle: 'Crossfader $label decks',
      childBuilder: (context, controller) => MTappable(
        semanticsLabel: 'Crossfader $label deck assignments',
        onPress: enabled ? controller.toggle : null,
        builder: (context, state) => SizedBox(
          width: width,
          height: _crossfaderFootHeight,
          child: DecoratedBox(
            decoration: BoxDecoration(
              color: state.active ? theme.hoverWash : null,
              borderRadius: theme.style.borderRadius.sm,
            ),
            child: Center(
              child: Row(
                mainAxisSize: .min,
                spacing: 2,
                children: [
                  Text(
                    label,
                    style: theme.typography.body.xs.copyWith(
                      color: enabled ? color : theme.colors.mutedForeground,
                      fontWeight: .w600,
                    ),
                  ),
                  Icon(
                    LucideIcons.chevronDown,
                    size: 10,
                    color: enabled ? color : theme.colors.mutedForeground,
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
      menuBuilder: (menuContext, controller) => Consumer(
        builder: (context, menuRef, _) {
          final enabled = menuRef.watch(engineRunningProvider);
          final numDecks =
              menuRef.watch(appSettingsProvider).value?.numDecks ?? 2;
          return Column(
            mainAxisSize: .min,
            crossAxisAlignment: .stretch,
            children: [
              for (var deckId = 0; deckId < numDecks; deckId++)
                Consumer(
                  builder: (context, deckRef, _) {
                    final selected =
                        deckRef
                            .watch(deckMixerChannelProvider(deckId))
                            .crossfaderAssign ==
                        side;
                    final deckLabel = String.fromCharCode(65 + deckId);
                    final deckInkColor = deckInk(
                      deckAccentFor(deckId)!,
                      context.theme.colors.brightness,
                    );
                    return Semantics(
                      label: 'Deck $deckLabel',
                      checked: selected,
                      selected: selected,
                      child: MixarMenuItem(
                        enabled: enabled,
                        title: ExcludeSemantics(
                          child: Row(
                            children: [
                              Expanded(
                                child: Text(
                                  'Deck $deckLabel',
                                  style: TextStyle(color: deckInkColor),
                                ),
                              ),
                              if (selected)
                                Icon(
                                  LucideIcons.check,
                                  size: 14,
                                  color: context.theme.colors.foreground,
                                ),
                            ],
                          ),
                        ),
                        onPress: () {
                          unawaited(
                            runMixerCommand(
                              menuContext,
                              () => setDeckCrossfaderAssign(
                                deckRef,
                                deckId,
                                selected ? CrossfaderAssign.thru : side,
                              ),
                            ),
                          );
                        },
                      ),
                    );
                  },
                ),
            ],
          );
        },
      ),
    );
  }
}
