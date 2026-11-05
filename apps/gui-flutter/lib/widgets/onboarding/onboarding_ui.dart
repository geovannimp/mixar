import 'package:flutter/widgets.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/widgets/shell/m_tappable.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Width at which a setup step switches from the stacked (narrow window) form
/// to the two-column booth layout.
const double kOnboardingWideBreakpoint = 900;

/// One setup step: a headline column and the step's own content.
///
/// Every step shares this so the flow keeps one rhythm — the headline lands in
/// the same place each time and only the content to its right changes.
class OnboardingBody extends StatelessWidget {
  const new({
    required this.eyebrow,
    required this.title,
    required this.description,
    required this.child,
    this.titleHighlight,
    this.childWidth = 620,
    this.emphasis = false,
    this.footnote,
    super.key,
  });

  /// Centred line under both columns (a closing note, not step content).
  final Widget? footnote;

  /// Renders the title at hero size (first step only).
  final bool emphasis;

  /// Uppercase kicker above the title (e.g. "STEP 2 — LOOK").
  final String eyebrow;
  final String title;

  /// Substring of [title] drawn in the accent (the product name).
  final String? titleHighlight;
  final String description;
  final Widget child;

  /// Cap for the content column; wide steps (library) pass a larger value.
  final double childWidth;

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (_, constraints) {
        final wide = constraints.maxWidth >= kOnboardingWideBreakpoint;
        final headline = _Headline(
          emphasis: emphasis,
          eyebrow: eyebrow,
          title: title,
          titleHighlight: titleHighlight,
          description: description,
        );
        final content = Center(
          child: ConstrainedBox(
            constraints: BoxConstraints(maxWidth: childWidth),
            child: child,
          ),
        );
        final note = footnote;
        if (!wide) {
          return SingleChildScrollView(
            padding: const EdgeInsets.fromLTRB(32, 24, 32, 24),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              spacing: 28,
              children: [
                headline,
                content,
                if (note != null) Center(child: note),
              ],
            ),
          );
        }
        return Padding(
          padding: const EdgeInsets.fromLTRB(56, 8, 56, 20),
          child: Column(
            children: [
              Expanded(
                child: Row(
                  spacing: 56,
                  children: [
                    SizedBox(width: 430, child: headline),
                    Expanded(
                      // Centre short content against the headline; taller
                      // content fills and scrolls.
                      child: Center(
                        child: SingleChildScrollView(child: content),
                      ),
                    ),
                  ],
                ),
              ),
              if (note != null)
                Padding(
                  padding: const EdgeInsets.only(top: 18),
                  child: Center(child: note),
                ),
            ],
          ),
        );
      },
    );
  }
}

class _Headline extends StatelessWidget {
  const new({
    required this.emphasis,
    required this.eyebrow,
    required this.title,
    required this.description,
    this.titleHighlight,
  });

  final bool emphasis;
  final String eyebrow;
  final String title;

  /// Substring of [title] drawn in the accent (the product name).
  final String? titleHighlight;
  final String description;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    // The step index and total are drawn by the page-level rail; the headline
    // itself no longer moves between pages.
    return _copy(theme);
  }

  /// Title text, with [titleHighlight] painted in the accent.
  TextSpan _titleSpan(MixarThemeData theme) {
    final highlight = titleHighlight;
    final start = highlight == null ? -1 : title.indexOf(highlight);
    if (highlight == null || start < 0) {
      return TextSpan(text: title);
    }
    return TextSpan(
      children: [
        TextSpan(text: title.substring(0, start)),
        TextSpan(
          text: highlight,
          style: TextStyle(color: theme.colors.accent),
        ),
        TextSpan(text: title.substring(start + highlight.length)),
      ],
    );
  }

  Widget _copy(MixarThemeData theme) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        Text(
          eyebrow.toUpperCase(),
          style: theme.typography.mono.xs.copyWith(
            color: theme.colors.accent,
            letterSpacing: 2.4,
            height: 1,
          ),
        ),
        const SizedBox(height: 18),
        Text.rich(
          _titleSpan(theme),
          style: emphasis
              ? theme.typography.display.xl5.copyWith(
                  fontWeight: FontWeight.w600,
                  height: 1.02,
                  letterSpacing: -1.6,
                )
              : theme.typography.display.xl4.copyWith(
                  fontWeight: FontWeight.w600,
                  height: 1.08,
                  letterSpacing: -1.1,
                ),
        ),
        const SizedBox(height: 14),
        Text(
          description,
          style: theme.typography.body.md.copyWith(
            color: theme.colors.mutedForeground,
            height: 1.6,
          ),
        ),
      ],
    );
  }
}

/// Vertical setup track beside the headline.
///
/// A full-height rail with the walked distance painted in accent, the current
/// stop lit and the rest waiting. The fill is its own animation: advancing a
/// step slides it down instead of jumping, so the run feels like a track
/// rather than a page counter.
class OnboardingProgressLane extends StatefulWidget {
  const new({required this.step, required this.total, super.key});

  final int step;
  final int total;

  @override
  State<OnboardingProgressLane> createState() => _OnboardingProgressLaneState();
}

class _OnboardingProgressLaneState extends State<OnboardingProgressLane> {
  /// Stop pitch and marker box; the spine runs through the markers' centre.
  static const _pitch = 30.0;
  static const _marker = 10.0;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final colors = theme.colors;
    final l10n = AppLocalizations.of(context)!;
    final step = widget.step;
    final total = widget.total;
    final walk = total <= 1 ? 1.0 : step / (total - 1);
    final dim = colors.mutedForeground.withValues(alpha: 0.55);
    // The rail has to be visible against the stage, unlike a card hairline.
    final railColor = colors.mutedForeground.withValues(alpha: 0.3);

    TextStyle number(Color color, FontWeight weight) =>
        theme.typography.mono.xs.copyWith(
          color: color,
          height: 1,
          letterSpacing: 0.8,
          fontWeight: weight,
        );

    return Semantics(
      container: true,
      label: l10n.onboardingStepProgress(step + 1, total),
      child: ExcludeSemantics(
        child: Stack(
          children: [
            // Rail: the whole track, behind the stops.
            Positioned(
              left: _marker / 2 - 1,
              top: _marker / 2,
              bottom: _marker / 2,
              child: SizedBox(width: 2, child: ColoredBox(color: railColor)),
            ),
            // Walked distance: its own animation, so advancing slides.
            Positioned(
              left: _marker / 2 - 1,
              top: _marker / 2,
              bottom: _marker / 2,
              child: AnimatedFractionallySizedBox(
                duration: const Duration(milliseconds: 420),
                curve: Curves.easeOutCubic,
                alignment: Alignment.topCenter,
                heightFactor: walk,
                child: SizedBox(
                  width: 2,
                  child: ColoredBox(color: colors.accent),
                ),
              ),
            ),
            Column(
              mainAxisSize: MainAxisSize.min,
              spacing: _pitch - _marker,
              children: [
                for (var i = 0; i < total; i++)
                  Row(
                    mainAxisSize: MainAxisSize.min,
                    spacing: 9,
                    children: [
                      SizedBox.square(
                        dimension: _marker,
                        child: Center(
                          child: AnimatedContainer(
                            duration: const Duration(milliseconds: 260),
                            curve: Curves.easeOutCubic,
                            width: i == step ? _marker : 5,
                            height: i == step ? _marker : 5,
                            decoration: BoxDecoration(
                              color: i == step
                                  ? colors.accent
                                  : i < step
                                  ? colors.accent.withValues(alpha: 0.8)
                                  : colors.background,
                              // Future stops are hollow, and every stop keeps
                              // the spine readable underneath it.
                              border: i > step
                                  ? Border.all(color: railColor)
                                  : null,
                            ),
                          ),
                        ),
                      ),
                      Text(
                        (i + 1).toString().padLeft(2, '0'),
                        style: number(
                          i == step
                              ? colors.accent
                              : i < step
                              ? colors.mutedForeground
                              : dim,
                          i == step ? FontWeight.w600 : FontWeight.w400,
                        ),
                      ),
                    ],
                  ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

/// Selectable setup option: icon, label, optional detail, and a check square.
///
/// Square chrome and a hairline border match the rest of the app; selection is
/// carried by the accent border plus an accent-tinted fill, not by a shadow.
class OnboardingChoiceCard extends StatelessWidget {
  const new({
    required this.title,
    required this.onPress,
    this.subtitle,
    this.icon,
    this.selected = false,
    this.semanticsLabel,
    this.stacked = false,
    this.compact = false,
    this.trailing,
    super.key,
  });

  /// Tighter vertical padding for single-line option rows.
  final bool compact;

  final String title;
  final String? subtitle;
  final IconData? icon;
  final bool selected;
  final VoidCallback? onPress;
  final String? semanticsLabel;

  /// Icon above the text instead of beside it (compact 3-up grids).
  final bool stacked;

  /// Extra widget under the text (e.g. a live-preview strip).
  final Widget? trailing;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final colors = theme.colors;
    return MTappable(
      onPress: onPress,
      selected: selected,
      semanticsLabel: semanticsLabel ?? title,
      builder: (_, state) {
        // A tinted fill reads as a solid block on a dark stage; an accent edge
        // plus a barely-there wash keeps the card's own surface.
        final fill = selected
            ? Color.alphaBlend(
                colors.primary.withValues(alpha: 0.10),
                colors.card,
              )
            : state.hovered
            ? colors.muted
            : colors.card;
        final borderColor = selected
            ? colors.primary
            : state.focused
            ? colors.accent
            : colors.border;
        final iconColor = selected ? colors.primary : colors.mutedForeground;
        return DecoratedBox(
          decoration: BoxDecoration(
            color: fill,
            border: Border.all(
              color: borderColor,
              width: selected ? 2 : theme.style.borderWidth,
            ),
          ),
          child: Padding(
            padding: EdgeInsets.symmetric(
              horizontal: 16,
              vertical: compact ? 11 : 16,
            ),
            child: stacked
                ? Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Row(
                        children: [
                          if (icon case final icon?)
                            Icon(icon, size: 18, color: iconColor),
                          const Spacer(),
                          _CheckSquare(selected: selected),
                        ],
                      ),
                      const SizedBox(height: 18),
                      _cardText(context),
                      if (trailing case final trailingWidget?) ...[
                        const SizedBox(height: 14),
                        trailingWidget,
                      ],
                    ],
                  )
                : Row(
                    spacing: 14,
                    children: [
                      if (icon case final iconData?)
                        Icon(iconData, size: 20, color: iconColor),
                      Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          mainAxisSize: MainAxisSize.min,
                          spacing: 3,
                          children: [_cardText(context), ?trailing],
                        ),
                      ),
                      _CheckSquare(selected: selected),
                    ],
                  ),
          ),
        );
      },
    );
  }

  Widget _cardText(BuildContext context) {
    final theme = context.theme;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      spacing: 3,
      children: [
        Text(
          title,
          style: theme.typography.body.md.copyWith(
            fontWeight: FontWeight.w600,
            height: 1.25,
          ),
        ),
        if (subtitle case final subtitleText?)
          Text(
            subtitleText,
            style: theme.typography.body.sm.copyWith(
              color: theme.colors.mutedForeground,
              height: 1.4,
            ),
          ),
      ],
    );
  }
}

class _CheckSquare extends StatelessWidget {
  const new({required this.selected});

  final bool selected;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    if (!selected) {
      return SizedBox.square(
        dimension: 18,
        child: DecoratedBox(
          decoration: BoxDecoration(
            border: Border.all(
              color: theme.colors.border,
              width: theme.style.borderWidth,
            ),
          ),
        ),
      );
    }
    return SizedBox.square(
      dimension: 18,
      child: ColoredBox(
        color: theme.colors.primary,
        child: Icon(
          LucideIcons.check,
          size: 12,
          color: theme.colors.primaryForeground,
        ),
      ),
    );
  }
}

/// A tip row: accent icon, title, muted body — no card chrome.
///
/// Closing tips read as guidance rather than as settings, so they stay
/// undecorated on the stage.
class OnboardingTipRow extends StatelessWidget {
  const new({
    required this.icon,
    required this.title,
    required this.subtitle,
    super.key,
  });

  final IconData icon;
  final String title;
  final String subtitle;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 6),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        spacing: 16,
        children: [
          Padding(
            padding: const EdgeInsets.only(top: 2),
            child: Icon(icon, size: 20, color: theme.colors.accent),
          ),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              spacing: 3,
              children: [
                Text(
                  title,
                  style: theme.typography.body.md.copyWith(
                    fontWeight: FontWeight.w600,
                    height: 1.25,
                  ),
                ),
                Text(
                  subtitle,
                  style: theme.typography.body.sm.copyWith(
                    color: theme.colors.mutedForeground,
                    height: 1.5,
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

/// Uppercase group label used inside a step's content column.
class OnboardingGroupLabel extends StatelessWidget {
  const new({required this.label, super.key});

  final String label;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return Text(
      label.toUpperCase(),
      style: theme.typography.mono.xs.copyWith(
        color: theme.colors.mutedForeground,
        letterSpacing: 1.8,
        height: 1,
      ),
    );
  }
}

/// Compact button for in-card actions (browse, retry).
class OnboardingSmallButton extends StatelessWidget {
  const new({required this.label, required this.icon, this.onPress, super.key});

  final String label;
  final IconData icon;
  final VoidCallback? onPress;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final colors = theme.colors;
    return MTappable(
      onPress: onPress,
      semanticsLabel: label,
      builder: (_, state) {
        final ink = colors.primaryForeground;
        final faded = onPress == null;
        final focused = state.focused;
        return Opacity(
          opacity: faded ? colors.disabledOpacity : 1,
          child: DecoratedBox(
            decoration: BoxDecoration(
              color: colors.primary,
              // Focus has to show: the fill is the same accent the stage uses.
              border: Border.all(
                color: focused ? colors.foreground : colors.primary,
                width: focused ? 2 : theme.style.borderWidth,
              ),
            ),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 9),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                spacing: 8,
                children: [
                  Text(
                    label,
                    style: theme.typography.body.sm.copyWith(
                      color: ink,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  Icon(icon, size: 14, color: ink),
                ],
              ),
            ),
          ),
        );
      },
    );
  }
}
