import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/utils/settings_search.dart';
import 'package:gui_flutter/widgets/settings/settings_insets.dart';
import 'package:gui_flutter/widgets/settings/settings_search_scope.dart';
import 'package:gui_flutter/widgets/settings/settings_searchable_card.dart';

/// Full-bleed settings section: the cards of one section, run to the pane
/// edges with their own top and bottom hairlines.
///
/// The section's title and description are not here: they live in the pinned
/// header above this body (see `SettingsSectionStickyHeader`), which stays on
/// screen while these cards scroll under it.
class SettingsSectionBody extends StatelessWidget {
  const new({required this.cards, this.before, super.key});

  /// Gap between stacked card bands.
  static const _cardGap = 16.0;

  /// Gap under a [before] slot, so it never breaks the card rhythm.
  static const _beforeGap = 24.0;

  /// Placed above the first card (e.g. a load error).
  final Widget? before;

  final List<Widget> cards;

  List<Widget> _visibleCards(BuildContext context, String foldedQuery) {
    if (foldedQuery.isEmpty) {
      return cards;
    }
    final matching = <Widget>[
      for (final card in cards)
        if (card is SettingsSearchableCard &&
            (card as SettingsSearchableCard).matchesSettingsSearch(
              context,
              foldedQuery,
            ))
          card,
    ];
    // The section itself matched the query; when no card advertises the term,
    // keep the whole section visible instead of showing an empty pane.
    return matching.isEmpty ? cards : matching;
  }

  @override
  Widget build(BuildContext context) {
    final before = this.before;
    final query = foldSettingsSearchText(
      SettingsSearchScope.queryOf(context).trim(),
    );
    final visibleCards = _visibleCards(context, query);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        if (before != null)
          Padding(
            padding: const EdgeInsets.fromLTRB(
              settingsCardSideInset,
              0,
              settingsCardSideInset,
              _beforeGap,
            ),
            child: before,
          ),
        Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          spacing: _cardGap,
          children: visibleCards,
        ),
      ],
    );
  }
}
