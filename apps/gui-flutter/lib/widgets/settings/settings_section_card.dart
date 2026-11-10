import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/utils/settings_search.dart';
import 'package:gui_flutter/widgets/settings/settings_insets.dart';
import 'package:gui_flutter/widgets/settings/settings_searchable_card.dart';
import 'package:gui_flutter/widgets/settings/settings_section_header.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// Vertical rhythm inside a card: outer padding, the header-to-body gap and the
/// breathing room kept below a revealed body.
const _cardInset = 16.0;

/// Key for the hairline a decorated card paints between its header and fields.
///
/// Tests use it to check the rule spans the band and disappears with the body.
const Key settingsCardRuleKey = ValueKey('settings-card-rule');

/// Inset shared by a card's header block and its fields.
///
/// The rule between the two runs full-bleed, so each block carries the inset
/// itself instead of one padded column wrapping both.
const _cardBlockPadding = EdgeInsets.symmetric(
  horizontal: settingsCardSideInset,
  vertical: _cardInset,
);

/// Room the collapsing body keeps outside its own box.
///
/// Fields paint their focus ring outside their bounds (shad inflates the ring
/// 4 px and strokes it 2 px wide), while the card only has to cut the body's
/// height. Without the bleed the side strokes fall outside the clip, and the
/// last field's bottom stroke is cut at the body's own bottom edge: a focused
/// field then reads as stray top lines.
const _ringBleed = 8.0;

/// Cuts a collapsing body's height, keeping [_ringBleed] on its sides and
/// under its last field.
class _BodyClip extends CustomClipper<Rect> {
  const new();

  @override
  Rect getClip(Size size) => Rect.fromLTRB(
    -_ringBleed,
    0,
    size.width + _ringBleed,
    size.height + _ringBleed,
  );

  @override
  bool shouldReclip(covariant _BodyClip oldClipper) => false;
}

/// Settings card body: required title + subtitle above optional [child].
///
/// Pass [trailing] for header actions (e.g. Update All, enable toggles). Omit
/// [child] when the card is header-only (toggle off, no fields to show): the
/// body then collapses with an animation, so toggling a header switch opens and
/// closes the card's config fields instead of snapping the list.
///
/// Full-bleed band: no side chrome, only a top and bottom hairline, so cards
/// stack as a striped list rather than separate boxes. It already owns its
/// edges — do not wrap it in `MCard`, which would double the hairlines.
class SettingsSectionCard extends StatelessWidget
    implements SettingsSearchableCard {
  const new({
    required this.title,
    this.subtitle = '',
    super.key,
    this.trailing,
    this.child,
    this.subtitleBuilder,
    this.titleBadge,
    this.decorated = true,
    this.searchTerms = const [],
  });

  /// Paint the band chrome (card fill + top/bottom hairlines).
  ///
  /// Setup steps show the same rows on the wizard's own stage, where a nested
  /// band would read as a card inside a card.
  final bool decorated;

  final String title;

  /// Muted description under the title. Empty leaves the card title-only.
  final String subtitle;

  /// Optional rich subtitle; see [SettingsSectionHeader.descriptionBuilder].
  final Widget Function(TextStyle style)? subtitleBuilder;
  final Widget? titleBadge;
  final Widget? trailing;
  final Widget? child;

  /// Field labels the card owns that are not in [title] or [subtitle].
  ///
  /// A section's `SettingsSection.searchTerms` surfaces the section for a
  /// field-label query; these keep the card that owns the field visible once
  /// the panel filters its cards, instead of showing only a sibling whose
  /// title happens to match.
  final List<String> searchTerms;

  @override
  bool matchesSettingsSearch(BuildContext context, String foldedQuery) =>
      settingsSearchMatches(
        '$title $subtitle ${searchTerms.join(' ')}',
        foldedQuery,
      );

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final header = SettingsSectionHeader(
      title: title,
      description: subtitle,
      descriptionBuilder: subtitleBuilder,
      badge: titleBadge,
    );
    final border = BorderSide(
      color: theme.colors.border,
      width: theme.style.borderWidth,
    );
    Widget headerRow() => trailing == null
        ? header
        : Row(
            crossAxisAlignment: .start,
            spacing: 12,
            children: [
              Expanded(child: header),
              trailing!,
            ],
          );

    if (!decorated) {
      return Padding(
        padding: const EdgeInsets.symmetric(vertical: _cardInset),
        child: Column(
          crossAxisAlignment: .start,
          children: [
            headerRow(),
            _CollapsibleCardBody(child: child),
          ],
        ),
      );
    }
    // Foreground border: the card fill would otherwise paint over the stroke,
    // and it costs no layout space between stacked cards.
    return DecoratedBox(
      position: DecorationPosition.foreground,
      decoration: BoxDecoration(
        border: Border(top: border, bottom: border),
      ),
      child: ColoredBox(
        color: theme.colors.card,
        // The header keeps its own inset so the rule under it can run the full
        // band width, matching the card's own hairlines.
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Padding(padding: _cardBlockPadding, child: headerRow()),
            _CollapsibleCardBody(
              showRule: true,
              // The rule spans the band, so the fields carry the band's side
              // and bottom inset themselves instead of the card padding them.
              contentPadding: _cardBlockPadding,
              child: child,
            ),
          ],
        ),
      ),
    );
  }
}

/// Animates the optional card body open and closed.
///
/// Keeps the last non-null body mounted while collapsing so the fields fade and
/// slide away instead of vanishing before the card shrinks. When the body is
/// absent the transition still sits in the tree at zero height, so the first
/// open animates too.
class _CollapsibleCardBody extends StatefulWidget {
  const new({this.child, this.contentPadding, this.showRule = false});

  final Widget? child;

  /// Space around the fields; the header already owns its own inset.
  ///
  /// Defaults to the top gap only, for cards that sit inside a padded stage.
  final EdgeInsets? contentPadding;

  /// Paints a hairline across the full body width, above the fields: the card
  /// header (title + subtitle) reads as its own band instead of running into
  /// the first field.
  ///
  /// The rule lives inside the collapsible body so a card whose fields are
  /// hidden shows no stray line, and it fades and shrinks with them.
  final bool showRule;

  @override
  State<_CollapsibleCardBody> createState() => _CollapsibleCardBodyState();
}

class _CollapsibleCardBodyState extends State<_CollapsibleCardBody>
    with SingleTickerProviderStateMixin {
  static const _duration = Duration(milliseconds: 200);

  late final AnimationController _controller;
  late final CurvedAnimation _curve;

  /// Body painted while open or collapsing; cleared once fully closed.
  Widget? _body;

  /// Set when a closed body opens, so only user-driven expansion scrolls.
  var _revealOnOpen = false;

  /// Body height used by the last auto reveal.
  ///
  /// Async fields (device lists, hints) can land after the open animation and
  /// grow the body past the viewport; a rebuild that changes this height gets
  /// one more nudge. Null until a user-driven open has revealed, so cards that
  /// merely mount open never scroll on rebuild.
  double? _revealedHeight;

  @override
  void initState() {
    super.initState();
    _body = widget.child;
    _controller = AnimationController(
      vsync: this,
      duration: _duration,
      value: widget.child == null ? 0 : 1,
    )..addStatusListener(_handleStatus);
    _curve = CurvedAnimation(
      parent: _controller,
      curve: Curves.easeOutCubic,
      reverseCurve: Curves.easeInCubic,
    );
  }

  @override
  void didUpdateWidget(covariant _CollapsibleCardBody oldWidget) {
    super.didUpdateWidget(oldWidget);
    final child = widget.child;
    if (child != null) {
      _body = child;
      if (oldWidget.child == null) {
        _revealOnOpen = true;
      } else if (_revealedHeight != null) {
        _scheduleRevealIfGrown();
      }
      _controller.forward();
    } else if (oldWidget.child != null) {
      _revealedHeight = null;
      _controller.reverse();
    }
  }

  void _handleStatus(AnimationStatus status) {
    if (status == AnimationStatus.completed) {
      if (_revealOnOpen) {
        _revealOnOpen = false;
        _revealOpenedBody();
      }
      return;
    }
    if (status != AnimationStatus.dismissed || _body == null) {
      return;
    }
    setState(() => _body = null);
  }

  /// Scrolls the surrounding page just enough to show the fields that just
  /// opened.
  ///
  /// Runs after the expansion so the reveal uses the final body height. Only a
  /// body whose end is past the viewport moves the page, and it moves just far
  /// enough to show that end.
  void _revealOpenedBody() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted || !_controller.isCompleted || widget.child == null) {
        return;
      }
      _reveal(opening: true);
    });
  }

  /// Re-reveals the body when content that loaded late made it taller.
  void _scheduleRevealIfGrown() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted || !_controller.isCompleted || widget.child == null) {
        return;
      }
      _reveal(opening: false);
    });
  }

  void _reveal({required bool opening}) {
    final scrollable = Scrollable.maybeOf(context);
    final box = context.findRenderObject();
    if (scrollable == null || box is! RenderBox || !box.hasSize) {
      return;
    }
    final position = scrollable.position;
    if (!position.hasContentDimensions) {
      return;
    }
    final height = box.size.height;
    final revealed = _revealedHeight;
    _revealedHeight = height;
    if (!opening && revealed != null && height <= revealed + 0.5) {
      return;
    }
    // A just-opened body may start below the fold and still needs revealing;
    // late growth, however, must not yank the page while the card is
    // off-screen.
    if (!opening) {
      final viewport = scrollable.context.findRenderObject();
      final top = viewport is RenderBox
          ? box.localToGlobal(Offset.zero, ancestor: viewport).dy
          : 0.0;
      if (top >= position.viewportDimension || top + height <= 0) {
        return;
      }
    }
    // Reveal the body's end plus [_cardInset], never scrolling back up: an
    // already visible body leaves the page alone, a cropped one scrolls just
    // enough to clear the last fields from the viewport edge. A body taller
    // than the viewport keeps its end visible too; the fields people reach for
    // when a switch turns on are the ones at the bottom.
    final viewport = RenderAbstractViewport.maybeOf(box);
    if (viewport == null) {
      return;
    }
    final reveal = viewport.getOffsetToReveal(box, 1).offset;
    // [_cardInset] keeps the last row off the viewport edge with the same
    // rhythm the card uses internally.
    final target = (reveal + _cardInset).clamp(
      position.minScrollExtent,
      position.maxScrollExtent,
    );
    if (target <= position.pixels) {
      return;
    }
    // [DrivenScrollActivity] asserts a positive duration, so reduced motion
    // jumps instead of animating.
    final duration = _controller.duration ?? _duration;
    if (duration == Duration.zero) {
      position.jumpTo(target);
      return;
    }
    position.animateTo(target, duration: duration, curve: Curves.easeOutCubic);
  }

  @override
  void dispose() {
    _curve.dispose();
    _controller.dispose();
    super.dispose();
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    // Motion is opt-out for users who ask the platform for less of it.
    _controller.duration = MediaQuery.disableAnimationsOf(context)
        ? Duration.zero
        : _duration;
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final body = _body;
    return ClipRect(
      clipper: const _BodyClip(),
      child: AnimatedBuilder(
        animation: _curve,
        builder: (context, child) => Align(
          // Bodies are form fields, so they line up with the header's title
          // instead of centering: an intrinsically narrow body (a status line,
          // a hint) would otherwise float in the middle of the card.
          alignment: Alignment.topLeft,
          heightFactor: _curve.value,
          child: child,
        ),
        child: FadeTransition(
          opacity: _curve,
          child: body == null
              ? const SizedBox(width: double.infinity)
              : ExcludeSemantics(
                  // Fields are already gone logically while they fade out.
                  excluding: widget.child == null,
                  child: IgnorePointer(
                    // Collapsing fields must not take taps.
                    ignoring: widget.child == null,
                    child: Column(
                      mainAxisSize: MainAxisSize.min,
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        if (widget.showRule)
                          // Full band width, like the card's own hairlines; the
                          // fields below keep their intrinsic width so a short
                          // body still hugs the left edge.
                          SizedBox(
                            key: settingsCardRuleKey,
                            width: double.infinity,
                            height: theme.style.borderWidth,
                            child: ColoredBox(color: theme.colors.border),
                          ),
                        Padding(
                          padding:
                              widget.contentPadding ??
                              const EdgeInsets.only(top: _cardInset),
                          child: body,
                        ),
                      ],
                    ),
                  ),
                ),
        ),
      ),
    );
  }
}
