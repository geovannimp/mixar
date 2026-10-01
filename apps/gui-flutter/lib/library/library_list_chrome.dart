import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/mixer/key_format.dart';
import 'package:gui_flutter/mixer/track_drag.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_providers.dart';
import 'package:gui_flutter/shell/app_button.dart';
import 'package:gui_flutter/shell/app_tooltip.dart';
import 'package:gui_flutter/shell/m_tabs.dart';
import 'package:gui_flutter/shell/mixar_input.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:super_drag_and_drop/super_drag_and_drop.dart';

/// Shared chrome for the library list panes (tracks, history): the filled
/// surface, the row density toggle, and the metadata pill / trailing meta
/// widgets both row layouts use.
///
/// Kept out of `track_list_pane.dart` so the history list renders the same
/// chips and BPM/key group rather than a second, drifting copy.

/// Focused-row fill: the selection tint blended over the list surface so it
/// reads as a lighter shade of the row rather than a separate grey.
Color libraryListSelectedRowColor(MixarThemeData theme) => Color.alphaBlend(
  theme.colors.primary.withValues(alpha: 0.14),
  theme.colors.card,
);

/// Horizontal gap between a list row's layout groups.
const kRowGutter = 8.0;

/// Narrowest a row's title may get before the trailing meta gives width back.
/// Below it the row would be all metadata and no name.
const kMinTitleWidth = 64.0;

/// Width of the trailing actions slot every list row reserves, so the status
/// overlay keeps its pills clear of the row's menu button.
const kActionsColumnWidth = 44.0;

/// Identifies the filter/sort toolbar, so a test can compare its height with
/// the sidebar tab bar's.
const kLibraryToolbarKey = ValueKey<String>('libraryToolbar');

/// Height of the library pane header — the sidebar tab bar and the filter/sort
/// toolbar — so the two rows line up across the split. Derived from the tab bar
/// itself ([kMTabBarHeight]) rather than restated as a literal, so the two
/// cannot drift.
const double kLibraryHeaderHeight = kMTabBarHeight;

/// Fill for the toolbar filter field: `secondary` nudged towards `card`, so the
/// field reads as a subtle darker inset on the toolbar instead of a full panel.
/// [focused] deepens it a step for a keyboard-focus cue, since the flat field
/// has no border to light up.
Color libraryToolbarFieldColor(MixarThemeData theme, {bool focused = false}) =>
    Color.alphaBlend(
      theme.colors.card.withValues(alpha: focused ? 0.75 : 0.5),
      theme.colors.secondary,
    );

/// Width for a row's trailing meta at [available] px: its natural size whenever
/// the row is wide enough, shrinking only once the title would drop below
/// [kMinTitleWidth]. [leading] is everything left of the title — the artwork and
/// its gap, or the history `#`/deck group — so both list layouts share one
/// budget instead of two copies that can drift.
///
/// Computed rather than left to flex, because `Expanded` title + `Flexible`
/// meta splits the free space evenly — the meta would claim half the row and
/// the title would wrap its pills at widths that have plenty of room.
double trailingMetaWidth(
  double available,
  double leading, {
  double natural = kTrailingMetaBaseWidth,
}) {
  final budget =
      available - leading - kRowGutter - kActionsColumnWidth - kMinTitleWidth;
  return budget.clamp(0.0, natural);
}

/// Identifies the right-hand BPM/key pill group, so a test can scope to it
/// rather than to the metadata pills on the left.
const kTrailingMetaKey = ValueKey<String>('libraryTrailingMeta');

// Maxima for the trailing pills, sized for the widest value plus its glyph and
// padding. They are maxima, not fixed widths: the group shrinks in a narrow
// pane instead of overflowing, and the pill labels ellipsize. Measured at
// body.xs: "109.7 BPM" is ~110px and a 3-char key ~37px, each plus 32px of
// pill chrome (padding + glyph + gap). Slack is deliberately small — these are
// maxima for a wide pane.
const kDurationSlotWidth = 84.0;
const kBpmSlotWidth = 144.0;
const kKeySlotWidth = 72.0;

// Flex weights matching the widths above. Equal flexes would split the meta
// evenly, starving the wider BPM pill and ellipsizing it even in a wide pane;
// weighting by natural width gives each its full size and shrinks them
// proportionally when the pane is narrow.
const kDurationSlotFlex = 84;
const kBpmSlotFlex = 144;
const kKeySlotFlex = 72;

/// Natural width of the trailing group without the duration pill.
const double kTrailingMetaBaseWidth =
    kBpmSlotWidth + kMetaPillGap + kKeySlotWidth;

/// Height reserved for pills: two runs of a 16px pill plus the 6px gap between
/// them, with a little slack for metric rounding. Sized from the actual pill
/// (body.xs line height + vertical padding), not a round number — reserving
/// more is what left the comfortable rows looking padded.
const kMetaPillAreaHeight = 40.0;

/// Gap between adjacent metadata pills.
const kMetaPillGap = 6.0;

/// Comfortable metadata pills: subtle chips under the title. The chip fill is
/// the row surface colour (`card`), so its background is invisible on a row —
/// the horizontal padding would read as a bare indent, which is why the pill
/// group is hung left by [kMetaPillInset] so its text lines up with the row
/// title.
const kMetaPillInset = 8.0;
const kMetaPillPadding = EdgeInsets.symmetric(
  horizontal: kMetaPillInset,
  vertical: 2,
);

/// Widest a pill may get before its padding leaves no room for a glyph.
const kMetaPillIconMinWidth = 40.0;

/// Centred message for a list pane's empty / loading / error states.
class LibraryListMessage extends StatelessWidget {
  const new(this.text, {required this.color, super.key});

  final String text;
  final Color color;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Text(
        text,
        style: context.theme.typography.body.sm.copyWith(color: color),
      ),
    );
  }
}

/// Filled surface behind a list pane's rows.
///
/// Flush and borderless: the list fills its pane edge to edge, so the row fill
/// is all this needs — no radius or border insets it from the panel edges.
class LibraryListSurface extends StatelessWidget {
  const new({required this.theme, required this.child, super.key});

  final MixarThemeData theme;
  final Widget child;

  @override
  Widget build(BuildContext context) =>
      ColoredBox(color: theme.colors.card, child: child);
}

/// The filter/sort header both library panes share: a header-height bar with a
/// bottom border, a flat filter field filling it, and the pane's own trailing
/// controls on the right.
///
/// Kept here (rather than copied into both panes) so the header height, border
/// and field chrome have a single definition and cannot drift.
class LibraryPaneToolbar extends StatefulWidget {
  const new({
    required this.hint,
    required this.onChanged,
    required this.trailing,
    super.key,
  });

  /// Placeholder for the filter field.
  final String hint;

  final ValueChanged<String> onChanged;

  /// Controls drawn after the field (sort menu, density toggle, actions).
  final List<Widget> trailing;

  @override
  State<LibraryPaneToolbar> createState() => _LibraryPaneToolbarState();
}

class _LibraryPaneToolbarState extends State<LibraryPaneToolbar> {
  final FocusNode _focusNode = FocusNode();

  @override
  void initState() {
    super.initState();
    _focusNode.addListener(_onFocusChange);
  }

  @override
  void dispose() {
    _focusNode.removeListener(_onFocusChange);
    _focusNode.dispose();
    super.dispose();
  }

  void _onFocusChange() => setState(() {});

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return ConstrainedBox(
      // minHeight, not a fixed height: the tab bar grows with its content, and
      // a hard box would overflow the field at larger text scales.
      constraints: const BoxConstraints(minHeight: kLibraryHeaderHeight),
      child: DecoratedBox(
        key: kLibraryToolbarKey,
        decoration: BoxDecoration(
          border: Border(
            bottom: BorderSide(
              color: theme.colors.border,
              width: theme.style.borderWidth,
            ),
          ),
        ),
        child: Padding(
          padding: const EdgeInsets.only(right: 6),
          child: Row(
            children: [
              Expanded(
                child: ColoredBox(
                  color: libraryToolbarFieldColor(
                    theme,
                    focused: _focusNode.hasFocus,
                  ),
                  // Centred vertically; the field itself takes the full width
                  // (`ShadInput`'s row expands), so the visible band and the
                  // tappable field coincide.
                  child: Center(
                    child: MixarInput(
                      hint: widget.hint,
                      borderless: true,
                      focusNode: _focusNode,
                      padding: const EdgeInsets.symmetric(
                        horizontal: 12,
                        vertical: 6,
                      ),
                      onChanged: widget.onChanged,
                    ),
                  ),
                ),
              ),
              const SizedBox(width: 6),
              ...widget.trailing,
            ],
          ),
        ),
      ),
    );
  }
}

/// Session-only density switch. Deliberately does not write settings.
class LibraryRowDensityButton extends ConsumerWidget {
  const new({required this.density, super.key});

  final LibraryRowDensity density;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final target = density.other;
    return AppTooltip(
      tip: target == LibraryRowDensity.compact
          ? 'Compact rows'
          : 'Comfortable rows',
      child: AppButton.icon(
        semanticsLabel: 'Toggle row layout',
        onPress: () => ref
            .read(libraryRowDensityOverrideProvider.notifier)
            .toggle(density),
        variant: .ghost,
        size: .xs,
        child: Icon(
          density.isCompact ? LucideIcons.list : LucideIcons.rows3,
          size: 14,
        ),
      ),
    );
  }
}

/// Right-aligned BPM + key, shared by the library list rows so their layouts
/// line up identically on the right. Fixed widths keep the values in columns
/// down the list; the title and pills absorb whatever width is left.
class TrailingMeta extends StatelessWidget {
  const new({
    required this.bpm,
    required this.rawKey,
    required this.keyDisplayMode,
    required this.keyColorMode,
    this.durationMs,
    super.key,
  });

  final double? bpm;
  final String rawKey;
  final KeyDisplayMode keyDisplayMode;
  final KeyColorMode keyColorMode;

  /// Rendered as a leading pill when set. Compact rows have no metadata pills
  /// row, so the trailing group is the only place their length can appear;
  /// comfortable rows already carry the duration on the left and pass null to
  /// avoid showing it twice.
  final int? durationMs;

  @override
  Widget build(BuildContext context) {
    // End-aligned so the group hugs the right edge and stays put when only some
    // of the pills are present.
    // Duration first, then BPM, then key: BPM and key stay flush right in both
    // densities, so they hold their position when the density is toggled.
    final duration = formatTrackDuration(durationMs);
    final children = <Widget>[
      if (duration.isNotEmpty)
        Flexible(
          flex: kDurationSlotFlex,
          child: MetaPill(
            text: duration,
            leading: const MetaPillGlyph(LucideIcons.clock),
          ),
        ),
      if (bpm != null)
        Flexible(
          flex: kBpmSlotFlex,
          child: MetaPill(
            text: '${bpm!.toStringAsFixed(1)} BPM',
            leading: const MetaPillGlyph(LucideIcons.metronome),
          ),
        ),
      if (rawKey.isNotEmpty)
        Flexible(
          flex: kKeySlotFlex,
          child: KeyPill(
            rawKey: rawKey,
            keyDisplayMode: keyDisplayMode,
            keyColorMode: keyColorMode,
          ),
        ),
    ];
    return Row(
      key: kTrailingMetaKey,
      mainAxisAlignment: MainAxisAlignment.end,
      children: [
        for (var i = 0; i < children.length; i++) ...[
          if (i > 0) const SizedBox(width: kMetaPillGap),
          children[i],
        ],
      ],
    );
  }
}

/// Muted pill glyph, sized for the metadata pills.
class MetaPillGlyph extends StatelessWidget {
  const new(this.icon, {super.key});

  final IconData icon;

  @override
  Widget build(BuildContext context) =>
      Icon(icon, size: 12, color: context.theme.colors.mutedForeground);
}

/// Key pill: the same chip as the rest of the metadata, with the key label
/// taking the configured key colour. Watches the harmonic reference so a deck
/// key change repaints the row without rebuilding the list.
class KeyPill extends ConsumerWidget {
  const new({
    required this.rawKey,
    required this.keyDisplayMode,
    required this.keyColorMode,
    super.key,
  });

  final String rawKey;
  final KeyDisplayMode keyDisplayMode;
  final KeyColorMode keyColorMode;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = context.theme;
    final reference = ref.watch(harmonicReferenceKeyProvider);
    final color = colorForKey(
      rawKey,
      keyColorMode,
      harmonicReferenceKey: reference,
    );
    return MetaPill(
      text: formatDeckKey(rawKey, keyDisplayMode),
      leading: const MetaPillGlyph(LucideIcons.music2),
      textColor: color ?? theme.colors.mutedForeground,
      fontWeight: color != null ? FontWeight.w600 : FontWeight.w500,
    );
  }
}

/// A metadata chip under a comfortable row's title.
class MetaPill extends StatelessWidget {
  const new({
    required this.text,
    this.leading,
    this.textColor,
    this.fontWeight = FontWeight.w500,
    super.key,
  });

  final String text;

  /// Optional leading widget, used by the metadata pills for their glyphs.
  final Widget? leading;

  /// Overrides the muted default, used by the key pill for its key colour.
  final Color? textColor;
  final FontWeight fontWeight;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return LayoutBuilder(
      builder: (context, constraints) {
        // Drop the glyph in a squeezed pill: below this the icon alone consumes
        // the whole box and pushes the label into an overflow. The text carries
        // the meaning, so the icon is the part that goes.
        final innerWidth = constraints.maxWidth - kMetaPillPadding.horizontal;
        final showLeading =
            leading != null && innerWidth >= kMetaPillIconMinWidth;
        return DecoratedBox(
          decoration: BoxDecoration(
            color: theme.colors.card,
            borderRadius: theme.style.borderRadius.pill,
          ),
          child: Padding(
            padding: kMetaPillPadding,
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                if (showLeading) ...[leading!, const SizedBox(width: 4)],
                // Flexible, not a bare Text: inside a min-size Row a loose
                // child keeps its intrinsic width and overflows the pill, so a
                // long artist name spills instead of ellipsizing.
                Flexible(
                  child: Text(
                    text,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: theme.typography.body.xs.copyWith(
                      color: textColor ?? theme.colors.mutedForeground,
                      fontWeight: fontWeight,
                    ),
                  ),
                ),
              ],
            ),
          ),
        );
      },
    );
  }
}

/// `m:ss` for [ms], or empty when the duration is unknown.
///
/// Empty, not `0:00`, for null and non-positive values: those come from missing
/// metadata, and the caller treats empty as "no duration to show". A *known*
/// sub-second length is not unknown, so it renders as `0:00` rather than
/// vanishing. Seconds are floored, not rounded, matching how file managers and
/// the pre-refactor table displayed the same values.
String formatTrackDuration(int? ms) {
  if (ms == null || ms <= 0) {
    return '';
  }
  final totalSec = ms ~/ 1000;
  final m = totalSec ~/ 60;
  final s = totalSec % 60;
  return '$m:${s.toString().padLeft(2, '0')}';
}

/// Card the pointer drags for a list row, shared by the tracks and history
/// lists so a drag out of either pane looks the same.
class LibraryDragCard extends StatelessWidget {
  const new({required this.title, super.key});

  final String title;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: theme.colors.background.withValues(alpha: 0.95),
        borderRadius: theme.style.borderRadius.md,
        border: Border.all(color: theme.colors.border),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 200),
          child: Text(
            title,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: theme.typography.body.sm.copyWith(
              fontWeight: FontWeight.w600,
            ),
          ),
        ),
      ),
    );
  }
}

/// Wraps a list row in the shared drag source. Both the tracks and history
/// lists drag a [TrackDragPayload] onto a deck, so the drag item, its platform
/// format and its drag card live here rather than in each pane.
Widget libraryTrackDragSource({
  required TrackDragPayload payload,
  required Widget child,
}) {
  return DragItemWidget(
    dragItemProvider: (_) async =>
        DragItem(localData: payload.toLocalData(), suggestedName: payload.title)
          ..add(Formats.plainText(encodeTrackDragPlainText(payload))),
    allowedOperations: () => [DropOperation.copy],
    dragBuilder: (context, _) => LibraryDragCard(title: payload.title),
    child: DraggableWidget(
      hitTestBehavior: HitTestBehavior.opaque,
      child: child,
    ),
  );
}
