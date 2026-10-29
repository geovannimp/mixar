import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/settings/settings_section_header.dart';

/// Full-bleed settings section: an inset header above cards that run to the
/// pane edges, each card carrying its own top and bottom hairline.
class SettingsSectionBody extends StatelessWidget {
  const new({
    required this.title,
    required this.description,
    required this.cards,
    this.before,
    super.key,
  });

  /// Matches the page's vertical scroll inset, so the header sits visually
  /// centred between the line above it and the first card.
  static const _headerGap = 24.0;

  /// Gap between stacked card bands.
  static const _cardGap = 16.0;

  final String title;
  final String description;

  /// Placed under the header, above the first card (e.g. a load error).
  final Widget? before;

  final List<Widget> cards;

  @override
  Widget build(BuildContext context) {
    final before = this.before;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        // Only the header keeps the horizontal inset; the cards are full-bleed.
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16),
          child: SettingsSectionHeader(title: title, description: description),
        ),
        const SizedBox(height: _headerGap),
        if (before != null)
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 0, 16, _headerGap),
            child: before,
          ),
        Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          spacing: _cardGap,
          children: cards,
        ),
      ],
    );
  }
}
