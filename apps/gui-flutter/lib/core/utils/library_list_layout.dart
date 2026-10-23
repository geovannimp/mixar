import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/utils/track_drag.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/widgets/library/library_drag_card.dart';
import 'package:gui_flutter/widgets/library/library_nav_row.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:gui_flutter/widgets/shell/m_tab_header.dart';

/// Library surface width at or above which the sidebar + content split shows.
/// Below it the surface pages: the sidebar first, then the content pane.
const double kLibraryWideMinWidth = 1024;

/// Focused-row fill — same token as sidebar [librarySelectedRowColor].
Color libraryListSelectedRowColor(MixarThemeData theme) =>
    librarySelectedRowColor(theme);

/// Focused-row fill for a list that does not own browser focus.
///
/// The cursor stays put (so a focus step back to this pane lands where it left
/// off) but drops the selection colour that marks the focused pane — with the
/// sidebar owning focus, a bright cursor in the track list would contradict it.
/// A solid [MixarColors.muted] recess rather than an alpha blend, so it does not
/// shift with the surface underneath.
Color libraryListCursorRowColor(MixarThemeData theme) => theme.colors.muted;

/// Whether a row paints a rating pill (ratings are 1-5; 0 means unrated).
bool trackShowsRating(int? rating) => rating != null && rating > 0;

/// Whether a row paints a colour-label circle.
bool trackShowsColorLabel(LibraryColorLabel? colorLabel) => colorLabel != null;

/// Horizontal gap between a list row's layout groups.
const kRowGutter = 8.0;

/// Narrowest a row's title may get before the trailing meta gives width back.
/// Below it the row would be all metadata and no name.
const kMinTitleWidth = 64.0;

/// Smallest width worth showing an artist beside a title. Below it the name is
/// dropped: the title keeps its full text instead of both being crippled.
const kMinArtistWidth = 40.0;

/// Natural single-line width of [text] in [style], for fit decisions.
///
/// A heuristic, but a faithful one: it merges the ambient [DefaultTextStyle]
/// the way [Text] does (a caller's style rarely carries every field, and the
/// inherited letter spacing and font size change the width), and honours the
/// ambient text scale, so a scaled UI measures like it paints.
double measureTextWidth(BuildContext context, String text, TextStyle style) {
  final effective = DefaultTextStyle.of(context).style.merge(style);
  final painter = TextPainter(
    text: TextSpan(text: text, style: effective),
    maxLines: 1,
    textDirection: Directionality.of(context),
    textScaler: MediaQuery.textScalerOf(context),
  )..layout();
  final width = painter.width;
  painter.dispose();
  return width;
}

/// Whether [widths] pack into [runs] runs of [available] width, the way a
/// [Wrap] lays them out: greedy, [gap] between neighbours, and an item wider
/// than a run takes a run of its own.
bool fitsInRuns(
  List<double> widths, {
  required double available,
  required int runs,
  double gap = kMetaPillGap,
}) {
  if (available <= 0 || runs <= 0) {
    return false;
  }
  var runsUsed = 0;
  // Width used in the current run; zero means the run is still empty.
  var used = 0.0;
  for (final width in widths) {
    if (width <= 0) {
      continue;
    }
    if (used == 0) {
      // An item wider than the run still occupies it, and ellipsizes there.
      used = width;
      runsUsed++;
      if (runsUsed > runs) {
        return false;
      }
      continue;
    }
    if (used + gap + width <= available) {
      used += gap + width;
      continue;
    }
    used = width;
    runsUsed++;
    if (runsUsed > runs) {
      return false;
    }
  }
  return true;
}

/// Width of the trailing actions slot every list row reserves, so trailing
/// meta and row actions stay aligned down the list.
const kActionsColumnWidth = 44.0;

/// Identifies the filter/sort toolbar, so a test can compare its height with
/// the sidebar tab bar's.
const kLibraryToolbarKey = ValueKey<String>('libraryToolbar');

/// Height of the library pane header — the sidebar tab bar and the filter/sort
/// toolbar — so the two rows line up across the split. Pinned (not a minimum)
/// so the field band can fill it; equals the tab bar's minimum height
/// ([kMTabBarMinHeight]) and stays aligned while a tab label stays within
/// [kMTabHeaderMinSize].
const double kLibraryHeaderHeight = kMTabBarMinHeight;

/// Fill for the toolbar filter field: the chrome (`secondary`) nudged towards
/// the text colour so the field reads as a subtle inset rather than a full
/// panel. The target is brightness-dependent: on the light theme the text
/// colour darkens the chrome, while on the dark theme it lightens it — the dark
/// step is larger because `secondary` sits close to the page background there.
/// [focused] deepens the step as a keyboard-focus cue, since the flat field has
/// no border to light up.
Color libraryToolbarFieldColor(MixarThemeData theme, {bool focused = false}) {
  final colors = theme.colors;
  final dark = colors.brightness == Brightness.dark;
  // Dark idle/focus used to be 0.10 / 0.16; nudged down a notch so the band
  // is only slightly darker than that previous wash.
  final alpha = focused ? (dark ? 0.06 : 0.12) : (dark ? 0.04 : 0.06);
  return Color.alphaBlend(
    colors.foreground.withValues(alpha: alpha),
    colors.secondary,
  );
}

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

/// A single rating digit plus the star glyph and pill chrome.
const kRatingSlotWidth = 40.0;

/// The filled colour circle: intrinsic, no label or padding.
const kColorSlotWidth = 16.0;

// Flex weights matching the widths above. Equal flexes would split the meta
// evenly, starving the wider BPM pill and ellipsizing it even in a wide pane;
// weighting by natural width gives each its full size and shrinks them
// proportionally when the pane is narrow.
const kDurationSlotFlex = 84;
const kBpmSlotFlex = 144;
const kKeySlotFlex = 72;
const kRatingSlotFlex = 40;

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

/// Leading glyph/loader (~12) + gap (4) inside a [MetaPill]. Shared with tests
/// that assert content-sized pill chrome width.
const kMetaPillLeadingExtent = 16.0;

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

/// Wraps a list row in the shared drag source. Both the tracks and history
/// lists drag a [TrackDragPayload] onto a deck, so the drag item and its drag
/// card live here rather than in each pane.
///
/// [onDragStart] runs as the drag starts and [onDragEnd] when it completes —
/// on a drop or a cancel. Both fire once per drag.
///
/// `childWhenDragging` keeps the row mounted for the whole drag: the drawer
/// tucks itself away while a drag is in flight, and the drag dies with its
/// source if the row leaves the tree.
Widget libraryTrackDragSource({
  required TrackDragPayload payload,
  required Widget child,
  VoidCallback? onDragStart,
  VoidCallback? onDragEnd,
}) {
  return Draggable<TrackDragPayload>(
    data: payload,
    // The whole row is the drag handle, padding and gaps included: the
    // plugin's drag source behaved the same way.
    hitTestBehavior: HitTestBehavior.opaque,
    dragAnchorStrategy: pointerDragAnchorStrategy,
    feedback: IgnorePointer(child: LibraryDragCard(title: payload.title)),
    childWhenDragging: child,
    onDragStarted: onDragStart,
    onDragEnd: (_) => onDragEnd?.call(),
    child: child,
  );
}

// --- Row layout ---
// Fixed trailing widths keep BPM / key aligned down the list in both
// densities, which is what made the old column table scannable. The title (and
// the pills in comfortable) take whatever is left. The trailing slot widths and
// the shared budget live in `library_list_chrome.dart`.
const kArtworkGap = 10.0;

/// Partition analysis / stem lanes and synthesise queued jobs.
///
/// Shared by in-flow [_TrackStatusPills] and the bars-only overlay so lane
/// order cannot drift between the two surfaces.
List<TrackProgressInfo> trackStatusInfos({
  required List<TrackProgressInfo> jobs,
  required bool analyzing,
  required bool stemsGenerating,
}) {
  // Queued work has no phase yet. Reuse the label vocabulary with a null
  // fraction so it reads indeterminate instead of inventing a percentage.
  // At most one job per lane, so at most two pills. Partitioned rather than
  // sorted: the two-lane comparator ties within a lane, and `List.sort` is
  // only stable below its insertion-sort threshold, so a sort could swap
  // same-lane pills between frames on a progress tick.
  final analysisLane = <TrackProgressInfo>[
    for (final job in jobs)
      if (!isStemProgressPhase(job.phase)) job,
    if (analyzing && !jobs.any((i) => !isStemProgressPhase(i.phase)))
      const TrackProgressInfo(phase: 'analyze'),
  ];
  final stemLane = <TrackProgressInfo>[
    for (final job in jobs)
      if (isStemProgressPhase(job.phase)) job,
    if (stemsGenerating && !jobs.any((i) => isStemProgressPhase(i.phase)))
      const TrackProgressInfo(phase: 'stems_queued'),
  ];
  return [...analysisLane, ...stemLane];
}
