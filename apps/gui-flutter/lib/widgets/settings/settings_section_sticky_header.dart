import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/settings/settings_insets.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// The section header pinned to the top of the settings pane.
///
/// It carries the section's Save button, so it has to stay reachable while the
/// cards scroll: it sheds its description and its breathing room as
/// `shrinkOffset` grows, ending as a single title row.
class SettingsSectionStickyHeader extends SliverPersistentHeaderDelegate {
  new({
    required this.title,
    required this.description,
    required this.theme,
    required this.textScaler,
    this.action,
  });

  final String title;

  /// One line under the title at rest, gone once the header is compact.
  final String description;

  /// Resolved by the page: the extents are asked for before the header is laid
  /// out, so the copy it renders cannot be measured first.
  final MixarThemeData theme;
  final TextScaler textScaler;

  /// Section action — the Save button — right-aligned in the title row.
  final Widget? action;

  /// Recomputed lazily: the extent getters are asked for during layout, and
  /// the page rebuilds the delegate whenever the draft or section changes.
  late final SettingsSectionHeaderMetrics _metrics =
      SettingsSectionHeaderMetrics(theme, textScaler, description);

  @override
  double get minExtent => _metrics.compactExtent;

  @override
  double get maxExtent => _metrics.expandedExtent;

  @override
  Widget build(
    BuildContext context,
    double shrinkOffset,
    bool overlapsContent,
  ) {
    final metrics = _metrics;
    return SettingsSectionStickyHeaderBody(
      title: title,
      description: description,
      action: action,
      metrics: metrics,
      compactness: metrics.compactness(shrinkOffset),
      // Cards are sliding under the header: the rule is what separates them.
      showDivider: overlapsContent || shrinkOffset > 0,
    );
  }

  @override
  bool shouldRebuild(covariant SettingsSectionStickyHeader oldDelegate) =>
      title != oldDelegate.title ||
      description != oldDelegate.description ||
      action != oldDelegate.action ||
      theme != oldDelegate.theme ||
      textScaler != oldDelegate.textScaler;
}

/// Vertical metrics of the pinned header, shared by the delegate's extents and
/// the content that has to fit them, so the two cannot drift apart.
///
/// Line boxes are `fontSize * height` scaled by the ambient [TextScaler], never
/// measured: the extents are needed before the header lays out.
class SettingsSectionHeaderMetrics {
  const new(this.theme, this.textScaler, this.description);

  final MixarThemeData theme;
  final TextScaler textScaler;

  /// Copy the header shows under its title; empty reserves no line at all.
  final String description;

  /// Breathing room above the title, at rest and once compact.
  static const expandedTop = 24.0;
  static const compactTop = 6.0;

  /// Gap under the title row, at rest and once compact.
  static const expandedBottom = 24.0;
  static const compactBottom = 6.0;

  /// Gap between the title row and the description.
  static const _titleGap = 4.0;

  /// The row is at least as tall as the section's Save button (`sm`).
  static const _minRowHeight = 32.0;

  double _line(TextStyle style) =>
      textScaler.scale((style.fontSize ?? 14) * (style.height ?? 1));

  double get titleRow {
    final line = _line(theme.typography.body.md);
    return line > _minRowHeight ? line : _minRowHeight;
  }

  /// Line box the description needs, or zero when the section has none.
  double get descriptionLine =>
      description.isEmpty ? 0 : _line(theme.typography.body.sm);

  /// Space between the title row and the description, gone with the copy.
  double get titleGap => description.isEmpty ? 0 : _titleGap;

  double get expandedExtent =>
      expandedTop + titleRow + titleGap + descriptionLine + expandedBottom;

  double get compactExtent => compactTop + titleRow + compactBottom;

  /// 0 at rest, 1 once the header has fully collapsed.
  double compactness(double shrinkOffset) {
    final span = expandedExtent - compactExtent;
    if (span <= 0) {
      return 0;
    }
    return (shrinkOffset / span).clamp(0.0, 1.0);
  }
}

/// Paints [SettingsSectionStickyHeader]'s content in the room the pinned
/// header currently gives it.
///
/// Every vertical value is mixed from expanded to compact by [compactness], so
/// the content's own height stays equal to the sliver's shrunken extent.
class SettingsSectionStickyHeaderBody extends StatelessWidget {
  const new({
    required this.title,
    required this.description,
    required this.metrics,
    required this.compactness,
    this.action,
    this.showDivider = false,
    super.key,
  });

  final String title;
  final String description;

  /// Measured once by the delegate: the body only mixes these values, so its
  /// height always equals the extent the sliver gave it.
  final SettingsSectionHeaderMetrics metrics;
  final double compactness;
  final Widget? action;
  final bool showDivider;

  @override
  Widget build(BuildContext context) {
    final theme = metrics.theme;
    final t = compactness.clamp(0.0, 1.0);
    double mix(double expanded, double compact) =>
        expanded + (compact - expanded) * t;
    final titleStyle = TextStyle.lerp(
      theme.typography.body.md.copyWith(fontWeight: FontWeight.w600),
      theme.typography.body.sm.copyWith(fontWeight: FontWeight.w600),
      t,
    )!;
    final descriptionStyle = theme.typography.body.sm.copyWith(
      color: theme.colors.mutedForeground,
    );
    // Promoted so the collection-`if` below can read it off a local; named
    // apart from the field so the two cannot be confused.
    final sectionAction = action;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: theme.colors.background,
        border: showDivider
            ? Border(bottom: BorderSide(color: theme.colors.border))
            : null,
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: settingsCardSideInset),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            SizedBox(
              height: mix(
                SettingsSectionHeaderMetrics.expandedTop,
                SettingsSectionHeaderMetrics.compactTop,
              ),
            ),
            SizedBox(
              height: metrics.titleRow,
              child: Row(
                children: [
                  Expanded(
                    child: Align(
                      alignment: Alignment.centerLeft,
                      child: Text(
                        title,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: titleStyle,
                      ),
                    ),
                  ),
                  if (sectionAction != null) ...[
                    const SizedBox(width: 12),
                    sectionAction,
                  ],
                ],
              ),
            ),
            SizedBox(height: mix(metrics.titleGap, 0)),
            // The description keeps its line box while it has room, clamped
            // into the space left as the header collapses beside the fading
            // copy.
            SizedBox(
              height: mix(metrics.descriptionLine, 0),
              child: ClipRect(
                child: OverflowBox(
                  alignment: Alignment.topLeft,
                  minHeight: metrics.descriptionLine,
                  maxHeight: metrics.descriptionLine,
                  child: Opacity(
                    opacity: (1 - 2 * t).clamp(0.0, 1.0),
                    child: Text(
                      description,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: descriptionStyle,
                    ),
                  ),
                ),
              ),
            ),
            SizedBox(
              height: mix(
                SettingsSectionHeaderMetrics.expandedBottom,
                SettingsSectionHeaderMetrics.compactBottom,
              ),
            ),
          ],
        ),
      ),
    );
  }
}
