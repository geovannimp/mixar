import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/theme/app_typography.dart';
import 'package:gui_flutter/core/utils/key_format.dart';
import 'package:gui_flutter/core/utils/pad_format.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/library/artwork_cache.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/mixer/deck_drop_hover.dart';
import 'package:gui_flutter/widgets/mixer/deck_key_lock_button.dart';
import 'package:gui_flutter/widgets/mixer/waveform/overview_strip.dart';
import 'package:gui_flutter/widgets/shell/m_card.dart';
import 'package:gui_flutter/widgets/shell/m_divider.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:skeletonizer/skeletonizer.dart';

/// Square cover thumbnail size in the deck track-info card.
const _artworkThumbSize = 64.0;

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
    return MCard(
      clipBehavior: .antiAlias,
      borderColor: dropHover ? theme.colors.primary : null,
      child: Column(
        mainAxisSize: .min,
        children: [
          IntrinsicHeight(
            child: Row(
              crossAxisAlignment: .stretch,
              children: [
                // Square cover. Fixed size so a large decoded image's
                // intrinsic size cannot expand the row.
                SizedBox.square(
                  dimension: _artworkThumbSize,
                  child: _ArtworkThumb(
                    bytes: artwork,
                    hasTrack: widget.hasTrack,
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
                    padding: const .symmetric(horizontal: 8, vertical: 4),
                    child: Column(
                      crossAxisAlignment: .stretch,
                      children: [
                        Row(
                          spacing: 8,
                          children: [
                            Expanded(
                              child: _DeckTitleArtist(
                                hasTrack: widget.hasTrack,
                                title: widget.title,
                                artist: artist,
                                skeleton: skeleton,
                              ),
                            ),
                            // Harmonic ref flips on play/pause — keep it off
                            // the parent so artwork / title do not rebuild.
                            _DeckKeyLockControl(
                              deckId: widget.deckId,
                              hasTrack: widget.hasTrack,
                              rawKey: lib?.key,
                              keyMode: keyMode,
                              keyColorMode: keyColorMode,
                            ),
                          ],
                        ),
                        // Playhead ticks must not rebuild artwork / title / key.
                        RepaintBoundary(
                          child: _DeckTimeRow(
                            deckId: widget.deckId,
                            hasTrack: widget.hasTrack,
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
    );
  }
}

class _DeckTitleArtist extends StatelessWidget {
  const new({
    required this.hasTrack,
    required this.skeleton,
    this.title,
    this.artist,
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
            style: theme.typography.display.xs.copyWith(
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
              : 'Drop or load a track',
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

class _DeckTimeRow extends ConsumerWidget {
  const new({required this.deckId, required this.hasTrack});

  final int deckId;
  final bool hasTrack;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = context.theme;
    final positionMs = ref.watch(deckPositionMsProvider(deckId));
    final durationMs = ref.watch(deckDurationMsProvider(deckId));
    return Row(
      children: [
        Text(
          formatDeckRemainingDisplay(hasTrack ? positionMs : null, durationMs),
          style: theme.typography.body.sm.copyWith(
            fontFamily: MixarFonts.notoSansMono,
            fontWeight: .w700,
            fontFeatures: const [
              FontFeature.tabularFigures(),
              FontFeature.slashedZero(),
            ],
          ),
        ),
        const Spacer(),
        Text(
          formatDeckTotalDisplay(durationMs),
          style: theme.typography.body.xs.copyWith(
            fontFamily: MixarFonts.notoSansMono,
            color: theme.colors.mutedForeground,
            fontFeatures: const [FontFeature.tabularFigures()],
          ),
        ),
      ],
    );
  }
}

class _ArtworkThumb extends StatelessWidget {
  const new({required this.bytes, required this.hasTrack});

  final Uint8List? bytes;
  final bool hasTrack;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final borderRadius = theme.style.borderRadius.md.copyWith(
      topRight: const Radius.circular(0),
      bottomRight: const Radius.circular(0),
      bottomLeft: const Radius.circular(0),
    );
    final disc = Icon(
      LucideIcons.disc3,
      size: 20,
      color: hasTrack ? theme.colors.mutedForeground : theme.colors.border,
    );
    return DecoratedBox(
      decoration: BoxDecoration(
        borderRadius: borderRadius,
        color: theme.colors.background,
      ),
      child: ClipRRect(
        borderRadius: borderRadius,
        child: bytes != null && bytes!.isNotEmpty
            ? Image.memory(
                bytes!,
                fit: .cover,
                errorBuilder: (_, _, _) => Center(child: disc),
              )
            : disc,
      ),
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
