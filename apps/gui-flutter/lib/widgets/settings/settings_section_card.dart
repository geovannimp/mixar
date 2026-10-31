import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/settings/settings_section_header.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// Vertical rhythm inside a card: outer padding, the header-to-body gap and the
/// breathing room kept below a revealed body.
const _cardInset = 16.0;

/// Horizontal room the collapsing body keeps outside its own box.
///
/// Fields paint their focus ring outside their bounds (shad puts the ring 4 px
/// out and strokes it 2 px wide), and the card only has to cut the body's
/// height. Without the bleed the ring's left and right strokes fall outside the
/// clip and the focused field reads as a pair of stray top and bottom lines.
const _ringBleed = 8.0;

/// Cuts a collapsing body's height while leaving [_ringBleed] either side.
class _BodyClip extends CustomClipper<Rect> {
  const new();

  @override
  Rect getClip(Size size) =>
      Rect.fromLTRB(-_ringBleed, 0, size.width + _ringBleed, size.height);

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
class SettingsSectionCard extends StatelessWidget {
  const new({
    required this.title,
    this.subtitle = '',
    super.key,
    this.trailing,
    this.child,
    this.subtitleBuilder,
  });

  final String title;

  /// Muted description under the title. Empty leaves the card title-only.
  final String subtitle;

  /// Optional rich subtitle; see [SettingsSectionHeader.descriptionBuilder].
  final Widget Function(TextStyle style)? subtitleBuilder;
  final Widget? trailing;
  final Widget? child;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final header = SettingsSectionHeader(
      title: title,
      description: subtitle,
      descriptionBuilder: subtitleBuilder,
    );
    final border = BorderSide(
      color: theme.colors.border,
      width: theme.style.borderWidth,
    );
    // Foreground border: the card fill would otherwise paint over the stroke,
    // and it costs no layout space between stacked cards.
    return DecoratedBox(
      position: DecorationPosition.foreground,
      decoration: BoxDecoration(
        border: Border(top: border, bottom: border),
      ),
      child: ColoredBox(
        color: theme.colors.card,
        child: Padding(
          padding: const EdgeInsets.all(_cardInset),
          child: Column(
            crossAxisAlignment: .start,
            children: [
              if (trailing == null)
                header
              else
                Row(
                  crossAxisAlignment: .start,
                  spacing: 12,
                  children: [
                    Expanded(child: header),
                    trailing!,
                  ],
                ),
              _CollapsibleCardBody(child: child),
            ],
          ),
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
  const new({this.child});

  final Widget? child;

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
    final body = _body;
    return ClipRect(
      clipper: const _BodyClip(),
      child: AnimatedBuilder(
        animation: _curve,
        builder: (context, child) => Align(
          alignment: Alignment.topCenter,
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
                    child: Padding(
                      padding: const EdgeInsets.only(top: _cardInset),
                      child: body,
                    ),
                  ),
                ),
        ),
      ),
    );
  }
}
