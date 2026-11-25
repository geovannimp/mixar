import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/theme/app_typography.dart';
import 'package:gui_flutter/core/utils/key_format.dart';
import 'package:gui_flutter/core/utils/pad_format.dart';
import 'package:gui_flutter/core/utils/scroll_physics.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/library/artwork_cache.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/mixer/deck_cover.dart';
import 'package:gui_flutter/widgets/mixer/deck_drop_hover.dart';
import 'package:gui_flutter/widgets/mixer/deck_key_lock_button.dart';
import 'package:gui_flutter/widgets/mixer/deck_toggles.dart';
import 'package:gui_flutter/widgets/mixer/track_end_blink.dart';
import 'package:gui_flutter/widgets/mixer/waveform/overview_strip.dart';
import 'package:gui_flutter/widgets/shell/m_card.dart';
import 'package:gui_flutter/widgets/shell/m_divider.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:silky_scroll/silky_scroll.dart';
import 'package:skeletonizer/skeletonizer.dart';

/// Square cover thumbnail size in the deck track-info card.
const _artworkThumbSize = 64.0;

/// Horizontal padding inside the card's text column.
const _titleRowPadding = 8.0;

/// Narrowest the title/artist gets before the toggle cluster starts scrolling.
const _minTitleWidth = 56.0;

/// Narrowest the toggle cluster viewport may get before the title gives way.
/// Below this the key chip and a neighbour cannot both be reached; at that
/// point the cluster is more useful than a title that has nothing to say.
/// Keep this low enough that the supported 1024px window (four decks, mixer
/// open, ~300px per deck) still gets its 56px title.
const _kMinClusterViewport = 48.0;

/// Space between the title column and the toggle cluster.
const _titleClusterGap = 8.0;

/// Artwork + title/artist/key stacked over remaining time and overview.
class DeckTrackInfo extends ConsumerStatefulWidget {
  const new({
    required this.deckId,
    required this.hasTrack,
    required this.title,
    super.key,
  });

  final int deckId;
  final bool hasTrack;
  final String? title;

  @override
  ConsumerState<DeckTrackInfo> createState() => _DeckTrackInfoState();
}

class _DeckTrackInfoState extends ConsumerState<DeckTrackInfo> {
  ProviderSubscription<String?>? _trackIdSub;

  @override
  void initState() {
    super.initState();
    // Prefetch on track load / swap only — not on every rebuild (playhead,
    // settings, etc.). fireImmediately covers the already-loaded deck case.
    _trackIdSub = ref.listenManual<String?>(
      deckTrackIdProvider(widget.deckId),
      (prev, next) {
        if (next != null) {
          unawaited(
            ref.read(artworkCacheProvider.notifier).ensureLoaded([next]),
          );
        }
      },
      fireImmediately: true,
    );
  }

  @override
  void dispose() {
    _trackIdSub?.close();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final trackId = ref.watch(deckTrackIdProvider(widget.deckId));
    final artwork = trackId == null
        ? null
        : ref.watch(artworkCacheProvider.select((m) => m[trackId]));
    final lib = ref.watch(deckLibraryTrackProvider(widget.deckId));
    final artist = lib?.artist;
    final keyMode = keyModeFromSettings(
      ref
          .watch(appSettingsProvider)
          .maybeWhen(
            data: (s) => s.keyDisplayMode,
            orElse: () => KeyDisplayModeSetting.musical,
          ),
    );
    final keyColorMode = keyColorModeFromSettings(
      ref
          .watch(appSettingsProvider)
          .maybeWhen(
            data: (s) => s.keyColorMode,
            orElse: () => KeyColorModeSetting.off,
          ),
    );
    final skeleton = ref.watch(deckSkeletonProvider(widget.deckId));
    final dropHover = DeckDropHover.of(context);

    // Drop target stays deck-wide; only this card takes the hover chrome.
    // In-bounds border (not an outward glow) so ClipRect cannot eat an edge.
    // Blinks as a whole once the track is about to end; a gentler floor than
    // the waveform's keeps the title and times readable through the blink.
    return LayoutBuilder(
      builder: (context, constraints) {
        // Room the toggle cluster may take before it starts scrolling; the
        // title keeps a readable minimum. Measured here because the card's
        // title row sits inside an [IntrinsicHeight], which cannot host a
        // LayoutBuilder of its own. At rest the card paints one hairline (the
        // cover's divider). Drop hover closes its side outline, so the card
        // reserves both side hairlines too. The gap separates the title from
        // the cluster.
        final contentWidth =
            constraints.maxWidth -
            _artworkThumbSize -
            theme.style.borderWidth * (dropHover ? 3 : 1) -
            _titleRowPadding * 2;
        final clusterRoom = contentWidth - _titleClusterGap;
        // The title keeps its floor only while the cluster still gets a
        // usable viewport. On a deck too narrow for both, the cluster takes
        // the row and the title degrades first: a zero-width scroller would
        // make every moved control unreachable.
        final titleFloor =
            (clusterRoom - _minTitleWidth) >= _kMinClusterViewport
            ? _minTitleWidth
            : 0.0;
        final controlsMax = (clusterRoom - titleFloor).clamp(
          0.0,
          constraints.maxWidth,
        );
        final compactTime = contentWidth < _kTimeRowFullMinWidth;
        return TrackEndBlink(
          deckId: widget.deckId,
          minOpacity: 0.7,
          child: MCard(
            clipBehavior: .antiAlias,
            sideBorders: false,
            topBorder: false,
            // The deck card runs flush to the window edge, where the window
            // frame paints over the card's outermost column. Keep the closed
            // hover outline one hairline inside so both outer decks show it
            // whole.
            borderColor: dropHover ? theme.colors.primary : null,
            outlineInset: EdgeInsets.symmetric(
              horizontal: theme.style.borderWidth,
            ),
            child: Column(
              mainAxisSize: .min,
              children: [
                IntrinsicHeight(
                  child: Row(
                    crossAxisAlignment: .stretch,
                    children: [
                      // Square cover — the deck's load surface. Fixed size so a
                      // large decoded image's intrinsic size cannot expand the
                      // row.
                      SizedBox.square(
                        dimension: _artworkThumbSize,
                        child: DeckCover(
                          deckId: widget.deckId,
                          hasTrack: widget.hasTrack,
                          artwork: artwork,
                        ),
                      ),
                      MDivider(
                        axis: .vertical,
                        padding: .zero,
                        color: theme.colors.border,
                        thickness: theme.style.borderWidth,
                      ),
                      Expanded(
                        child: Padding(
                          padding: const .symmetric(
                            horizontal: _titleRowPadding,
                            vertical: 4,
                          ),
                          child: Column(
                            crossAxisAlignment: .stretch,
                            mainAxisAlignment: .spaceBetween,
                            children: [
                              Row(
                                spacing: _titleClusterGap,
                                children: [
                                  Expanded(
                                    child: _DeckTitleArtist(
                                      key: const ValueKey('deck-title-artist'),
                                      hasTrack: widget.hasTrack,
                                      title: widget.title,
                                      artist: artist,
                                      skeleton: skeleton,
                                    ),
                                  ),
                                  ConstrainedBox(
                                    constraints: BoxConstraints(
                                      maxWidth: controlsMax,
                                    ),
                                    child: SilkySingleChildScrollView(
                                      physics: mixarScrollPhysics(context),
                                      decayLogFactor: kMixarSilkyDecayLogFactor,
                                      blockWebOverscrollBehaviorX:
                                          kMixarSilkyBlockWebOverscrollBehaviorX,
                                      mouseWheelVerticalDeltaBehavior:
                                          MouseWheelVerticalDeltaBehavior
                                              .shiftOnly,
                                      scrollDirection: Axis.horizontal,
                                      reverse: true,
                                      child: Row(
                                        mainAxisSize: MainAxisSize.min,
                                        spacing: 4,
                                        children: [
                                          DeckToggles(deckId: widget.deckId),
                                          // Harmonic ref flips on play/pause — keep
                                          // it off the parent so artwork / title do
                                          // not rebuild.
                                          _DeckKeyLockControl(
                                            deckId: widget.deckId,
                                            hasTrack: widget.hasTrack,
                                            rawKey: lib?.key,
                                            keyMode: keyMode,
                                            keyColorMode: keyColorMode,
                                          ),
                                        ],
                                      ),
                                    ),
                                  ),
                                ],
                              ),
                              // Playhead ticks must not rebuild artwork / title / key.
                              RepaintBoundary(
                                child: _DeckTimeRow(
                                  deckId: widget.deckId,
                                  hasTrack: widget.hasTrack,
                                  compact: compactTime,
                                ),
                              ),
                            ],
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
                MDivider(
                  padding: .zero,
                  color: theme.colors.border,
                  thickness: theme.style.borderWidth,
                ),
                RepaintBoundary(
                  child: OverviewStrip(deckId: widget.deckId, height: 36),
                ),
              ],
            ),
          ),
        );
      },
    );
  }
}

class _DeckTitleArtist extends StatelessWidget {
  const new({
    required this.hasTrack,
    required this.skeleton,
    this.title,
    this.artist,
    super.key,
  });

  final bool hasTrack;
  final bool skeleton;
  final String? title;
  final String? artist;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final artistName = artist;
    return Column(
      crossAxisAlignment: .start,
      mainAxisSize: .min,
      children: [
        Skeletonizer(
          enabled: skeleton,
          child: Text(
            title ?? 'No track loaded',
            maxLines: 1,
            overflow: .ellipsis,
            style: theme.typography.display.sm.copyWith(
              fontFamily: MixarFonts.spaceGrotesk,
              fontWeight: .w700,
              height: 1.15,
              letterSpacing: -0.02,
            ),
          ),
        ),
        Text(
          hasTrack
              ? (artistName == null || artistName.isEmpty ? '—' : artistName)
              : 'Drop a track or click the cover',
          maxLines: 1,
          overflow: .ellipsis,
          style: theme.typography.body.xs.copyWith(
            color: theme.colors.mutedForeground,
          ),
        ),
      ],
    );
  }
}

/// Content width below which the deck's total time gives way to the remaining
/// time, so a narrow deck keeps its readout inside the card.
const _kTimeRowFullMinWidth = 108.0;

class _DeckTimeRow extends ConsumerWidget {
  const new({
    required this.deckId,
    required this.hasTrack,
    required this.compact,
  });

  final int deckId;
  final bool hasTrack;

  /// No room for the total next to the remaining time.
  final bool compact;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = context.theme;
    final positionMs = ref.watch(deckPositionMsProvider(deckId));
    final durationMs = ref.watch(deckDurationMsProvider(deckId));
    return Row(
      children: [
        Expanded(
          child: Text(
            compact
                ? formatDeckRemainingCompact(
                    hasTrack ? positionMs : null,
                    durationMs,
                  )
                : formatDeckRemainingDisplay(
                    hasTrack ? positionMs : null,
                    durationMs,
                  ),
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style:
                (compact ? theme.typography.body.xs : theme.typography.body.sm)
                    .copyWith(
                      fontFamily: MixarFonts.notoSansMono,
                      fontWeight: .w700,
                      fontFeatures: const [
                        FontFeature.tabularFigures(),
                        FontFeature.slashedZero(),
                      ],
                    ),
          ),
        ),
        if (!compact) ...[
          const SizedBox(width: 4),
          Text(
            formatDeckTotalDisplay(durationMs),
            style: theme.typography.body.xs.copyWith(
              fontFamily: MixarFonts.notoSansMono,
              color: theme.colors.mutedForeground,
              fontFeatures: const [FontFeature.tabularFigures()],
            ),
          ),
        ],
      ],
    );
  }
}

class _DeckKeyLockControl extends ConsumerWidget {
  const new({
    required this.deckId,
    required this.hasTrack,
    required this.rawKey,
    required this.keyMode,
    required this.keyColorMode,
  });

  final int deckId;
  final bool hasTrack;
  final String? rawKey;
  final KeyDisplayMode keyMode;
  final KeyColorMode keyColorMode;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final keyLock = ref.watch(deckKeyLockProvider(deckId));
    final keyShift = ref.watch(deckKeyShiftProvider(deckId)).round();
    final engineRunning = ref.watch(engineRunningProvider);
    final harmonicReference = ref.watch(harmonicReferenceKeyProvider);
    // Show the sounding key: the analyzed key transposed by the session key
    // shift. Display-only — never persisted.
    final soundingKey = transposeKey(rawKey, keyShift) ?? rawKey;
    final key = deckSoundingKeyLabel(rawKey, keyShift, keyMode);
    final keyColor = hasTrack
        ? colorForKey(
            soundingKey,
            keyColorMode,
            harmonicReferenceKey: harmonicReference,
          )
        : null;
    return DeckKeyLockButton(
      keyLabel: hasTrack ? key : '—',
      keyColor: keyColor,
      keyLock: keyLock,
      enabled: hasTrack && engineRunning,
      onChanged: (enabled) {
        unawaited(deckSetKeyLock(context, ref, deckId, enabled));
      },
    );
  }
}
