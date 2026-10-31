import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/theme/deck_accent.dart';
import 'package:gui_flutter/core/theme/mixar_accent.dart';
import 'package:gui_flutter/widgets/library/history_row_layout.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// Deck A/B, tinted with the fader accent so it matches the deck panels.
class DeckBadge extends StatelessWidget {
  const new({required this.deck});

  final int deck;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final accent = deckAccentFor(deck);
    final color = accent == null
        ? theme.colors.mutedForeground
        : textColorStep(deckTone(accent), theme.colors.brightness);
    return Semantics(
      label: deckDisplayLabel(deck),
      container: true,
      // Announce "Deck B", not the bare letter plus the label again.
      excludeSemantics: true,
      child: SizedBox(
        width: kHistoryDeckWidth,
        child: Text(
          historyDeckLetter(deck),
          textAlign: TextAlign.center,
          maxLines: 1,
          style: theme.typography.body.sm.copyWith(
            color: color,
            fontWeight: FontWeight.w700,
          ),
        ),
      ),
    );
  }
}

String historyDeckLetter(int deck) => switch (deck) {
  0 => 'A',
  1 => 'B',
  _ => '${deck + 1}',
};
