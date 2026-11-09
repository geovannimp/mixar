import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/mixer/layout/mixer_content.dart';
import 'package:gui_flutter/widgets/mixer/layout/mixer_plan.dart';
import 'package:gui_flutter/widgets/mixer/layout/mixer_slot.dart';
import 'package:gui_flutter/widgets/shell/m_divider.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:panes/panes.dart';

/// The mixer body: two fixed pane stacks whose slots hold whatever the plan
/// puts in them.
///
/// The outer stack is the band, the deck row, the bottom slot and the FX bar;
/// the deck row is a nested horizontal stack of deck(s) | centre | deck(s).
/// Splits are invisible — the rules the body needs are painted by the slots
/// themselves — so the only chrome is the hairline at a slot's leading edge.
class MixerPanes extends StatelessWidget {
  const new({
    required this.main,
    required this.content,
    required this.plan,
    super.key,
  });

  /// Outer stack (vertical). Sizes are session-local; the page solves them.
  final PaneController main;

  /// The deck row (horizontal).
  final PaneController content;

  /// What each slot holds, for the layout the page solved.
  final MixerPlan plan;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return ColoredBox(
      color: theme.colors.card,
      child: PaneTheme(
        // Invisible chrome; keep a grab hit-area (was library divider:none).
        data: const PaneThemeData(
          resizerColor: Color(0x00000000),
          resizerHoverColor: Color(0x00000000),
          resizerFocusedColor: Color(0x00000000),
          resizerThickness: 0,
          resizerHitTestThickness: 8,
        ),
        child: MultiPane(
          direction: Axis.vertical,
          controller: main,
          paneBuilder: (context, id, _) => _mainSlot(context, id),
        ),
      ),
    );
  }

  Widget _mainSlot(BuildContext context, String id) {
    final hasContent = plan.content != null;
    return switch (id) {
      MixerPaneIds.top => _frame(
        context,
        id,
        buildMixerContent(context, plan.top?.content),
        leadingRule: false,
      ),
      MixerPaneIds.content => _frame(
        context,
        id,
        _contentRow(),
        leadingRule: plan.top != null,
      ),
      MixerPaneIds.bottom => _frame(
        context,
        id,
        buildMixerContent(context, plan.bottom?.content),
        leadingRule: plan.top != null || hasContent,
      ),
      MixerPaneIds.bar => _frame(
        context,
        id,
        buildMixerContent(context, plan.bar?.content),
        leadingRule: plan.top != null || hasContent || plan.bottom != null,
      ),
      _ => const SizedBox.shrink(),
    };
  }

  /// Decks | centre | decks. The centre pane is pinned by the page; the decks
  /// share what is left.
  ///
  /// Takes no context of its own: each pane builds with the context `MultiPane`
  /// hands its builder, so there is nothing to shadow.
  Widget _contentRow() => MultiPane(
    key: const ValueKey('mixer-content-row'),
    direction: Axis.horizontal,
    controller: content,
    paneBuilder: (context, id, _) {
      final row = plan.content;
      final spec = switch (id) {
        MixerPaneIds.left => row?.left,
        MixerPaneIds.center => row?.center,
        MixerPaneIds.right => row?.right,
        _ => null,
      };
      return _frame(
        context,
        id,
        buildMixerContent(context, spec?.content),
        // With no centre column the decks meet on one rule, drawn over the
        // right deck's leading edge.
        leadingRule: id == MixerPaneIds.right && (row?.seam ?? false),
        axis: Axis.vertical,
      );
    },
  );

  /// The slot's content, with the rule that closes the slot before it painted
  /// over the content's leading edge. The rule does not consume layout, so the
  /// keyed content keeps the slot's full extent.
  Widget _frame(
    BuildContext context,
    String id,
    Widget child, {
    required bool leadingRule,
    Axis axis = Axis.horizontal,
  }) {
    if (!leadingRule) {
      return child;
    }
    final horizontal = axis == Axis.horizontal;
    return Stack(
      fit: StackFit.expand,
      children: [
        Positioned.fill(child: child),
        Positioned(
          left: 0,
          right: horizontal ? 0 : null,
          top: 0,
          bottom: horizontal ? null : 0,
          child: IgnorePointer(
            child: MDivider(
              key: ValueKey('mixer-rule-${id.split('.').last}'),
              axis: axis,
              padding: EdgeInsets.zero,
              color: context.theme.colors.border,
            ),
          ),
        ),
      ],
    );
  }
}
