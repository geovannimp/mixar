import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

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
