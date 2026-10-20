import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/utils/library_list_layout.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// A library row's identity line: the title, then the artist after a bullet.
///
/// The title owns the line. Its full text is measured first: the artist appears
/// only when the title still fits complete and leaves [kMinArtistWidth] beside
/// it, so a long title is never ellipsized to make room for the artist — the
/// artist is dropped instead.
///
/// Baseline aligned rather than centre aligned: the artist is set smaller, and
/// centring the two boxes leaves their text baselines a pixel or two apart.
class TrackTitleLine extends StatelessWidget {
  const new({
    required this.title,
    required this.artist,
    required this.titleStyle,
    super.key,
  });

  final String title;

  /// Empty hides the separator and the artist.
  final String artist;

  /// The title's text style; the artist's is derived from the theme, so both
  /// densities render it the same way.
  final TextStyle titleStyle;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final artistStyle = theme.typography.body.xs.copyWith(
      color: theme.colors.mutedForeground,
      fontWeight: FontWeight.w500,
    );
    return LayoutBuilder(
      builder: (context, constraints) {
        final available = constraints.maxWidth;
        final titleWidth = measureTextWidth(context, title, titleStyle);
        // Everything the artist costs before its own text.
        final leader =
            2 * kMetaPillGap + measureTextWidth(context, '•', artistStyle);
        final artistRoom = available - titleWidth - leader;
        // Hide the artist first: a title that has to ellipsize gets the whole
        // line rather than sharing it.
        final showArtist =
            artist.isNotEmpty &&
            titleWidth <= available &&
            artistRoom >= kMinArtistWidth;
        return Row(
          crossAxisAlignment: CrossAxisAlignment.baseline,
          textBaseline: TextBaseline.alphabetic,
          children: [
            Flexible(
              child: Text(
                title,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: titleStyle,
              ),
            ),
            if (showArtist) ...[
              const SizedBox(width: kMetaPillGap),
              Text('•', style: artistStyle),
              const SizedBox(width: kMetaPillGap),
              ConstrainedBox(
                constraints: BoxConstraints(maxWidth: artistRoom),
                child: Text(
                  artist,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: artistStyle,
                ),
              ),
            ],
          ],
        );
      },
    );
  }
}
