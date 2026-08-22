import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/library/artwork_cache.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/mixer/key_format.dart';
import 'package:gui_flutter/mixer/pad_format.dart';
import 'package:gui_flutter/mixer/rotary_knob.dart';
import 'package:gui_flutter/mixer/waveform/overview_strip.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:skeletonizer/skeletonizer.dart';

/// Artwork + title/artist/key stacked over remaining time and overview.
class DeckTrackInfo extends ConsumerStatefulWidget {
  const DeckTrackInfo({
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
  var _keyMode = KeyDisplayMode.musical;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final trackId = ref.watch(deckTrackIdProvider(widget.deckId));
    if (trackId != null) {
      unawaited(
        ref.read(artworkCacheProvider.notifier).ensureLoaded([trackId]),
      );
    }
    final artwork = trackId == null
        ? null
        : ref.watch(artworkCacheProvider)[trackId];
    final lib = ref.watch(deckLibraryTrackProvider(widget.deckId));
    final artist = lib?.artist;
    final key = formatDeckKey(lib?.key, _keyMode);
    final skeleton = ref.watch(deckSkeletonProvider(widget.deckId));
    final durationMs = ref.watch(deckDurationMsProvider(widget.deckId));
    final positionMs = ref.watch(deckPositionMsProvider(widget.deckId));

    return FCard(
      clipBehavior: .antiAlias,
      child: Column(
        mainAxisSize: .min,
        children: [
          IntrinsicHeight(
            child: Row(
              crossAxisAlignment: .stretch,
              children: [
                AspectRatio(
                  aspectRatio: 1,
                  child: _ArtworkThumb(
                    bytes: artwork,
                    hasTrack: widget.hasTrack,
                  ),
                ),
                FDivider(
                  axis: .vertical,
                  style: .delta(padding: .value(.all(0))),
                ),
                Expanded(
                  child: Padding(
                    padding: const .symmetric(horizontal: 8, vertical: 4),
                    child: Column(
                      crossAxisAlignment: .stretch,
                      mainAxisAlignment: .start,
                      children: [
                        Row(
                          children: [
                            Expanded(
                              child: _DeckTitleArtist(
                                hasTrack: widget.hasTrack,
                                title: widget.title,
                                artist: artist,
                                skeleton: skeleton,
                              ),
                            ),
                            GestureDetector(
                              onTap: !widget.hasTrack
                                  ? null
                                  : () => setState(() {
                                      _keyMode =
                                          _keyMode == KeyDisplayMode.musical
                                          ? KeyDisplayMode.camelot
                                          : KeyDisplayMode.musical;
                                    }),
                              child: Padding(
                                padding: const .symmetric(
                                  horizontal: 4,
                                  vertical: 2,
                                ),
                                child: Text(
                                  widget.hasTrack ? key : '—',
                                  style: theme.typography.body.xs.copyWith(
                                    color: theme.colors.mutedForeground,
                                    fontWeight: .w600,
                                    fontFeatures: const [
                                      FontFeature.tabularFigures(),
                                    ],
                                  ),
                                ),
                              ),
                            ),
                          ],
                        ),
                        _DeckTimeRow(
                          hasTrack: widget.hasTrack,
                          positionMs: positionMs,
                          durationMs: durationMs,
                        ),
                      ],
                    ),
                  ),
                ),
              ],
            ),
          ),
          FDivider(style: .delta(padding: .value(.all(0)))),
          OverviewStrip(deckId: widget.deckId, height: 36),
        ],
      ),
    );
  }
}

class _DeckTitleArtist extends StatelessWidget {
  const _DeckTitleArtist({
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
      mainAxisAlignment: .start,
      mainAxisSize: .min,
      children: [
        Skeletonizer(
          enabled: skeleton,
          child: Text(
            title ?? 'No track loaded',
            maxLines: 1,
            overflow: .ellipsis,
            style: theme.typography.body.sm.copyWith(fontWeight: .w700),
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

class _DeckTimeRow extends StatelessWidget {
  const _DeckTimeRow({
    required this.hasTrack,
    required this.positionMs,
    required this.durationMs,
  });

  final bool hasTrack;
  final int positionMs;
  final int? durationMs;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return Row(
      children: [
        Text(
          formatDeckRemainingDisplay(hasTrack ? positionMs : null, durationMs),
          style: theme.typography.body.sm.copyWith(
            fontWeight: .w600,
            fontFeatures: const [FontFeature.tabularFigures()],
          ),
        ),
        const Spacer(),
        Text(
          formatDeckTotalDisplay(durationMs),
          style: theme.typography.body.xs.copyWith(
            color: theme.colors.mutedForeground,
            fontFeatures: const [FontFeature.tabularFigures()],
          ),
        ),
      ],
    );
  }
}

class _ArtworkThumb extends StatelessWidget {
  const _ArtworkThumb({required this.bytes, required this.hasTrack});

  final Uint8List? bytes;
  final bool hasTrack;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final borderRadius = theme.style.borderRadius.md.copyWith(
      topRight: Radius.circular(0),
      bottomRight: Radius.circular(0),
      bottomLeft: Radius.circular(0),
    );
    return DecoratedBox(
      decoration: BoxDecoration(
        borderRadius: borderRadius,
        color: theme.colors.background,
      ),
      child: ClipRRect(
        borderRadius: borderRadius,
        child: bytes != null && bytes!.isNotEmpty
            ? Image.memory(bytes!, fit: .cover)
            : Icon(
                FLucideIcons.disc3,
                size: 20,
                color: hasTrack
                    ? theme.colors.mutedForeground
                    : theme.colors.border,
              ),
      ),
    );
  }
}

/// LUFS / ReplayGain / auto / trim popover (Tauri `DeckInfoPopover`).
class DeckGainPopover extends ConsumerWidget {
  const DeckGainPopover({
    required this.deckId,
    required this.hasTrack,
    super.key,
  });

  final int deckId;
  final bool hasTrack;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = context.theme;
    final loudness = ref.watch(deckLoudnessLufsProvider(deckId));
    final autoGain = ref.watch(deckAutoGainDbProvider(deckId));
    final trimDb = normToStripDb(
      ref.watch(deckMixerChannelProvider(deckId)).gainTrim,
    );
    final total = autoGain + trimDb;

    return FPopover(
      popoverAnchor: Alignment.topLeft,
      childAnchor: Alignment.bottomLeft,
      popoverBuilder: (context, _) {
        return ConstrainedBox(
          constraints: const BoxConstraints(minWidth: 180),
          child: Padding(
            padding: const .all(12),
            child: Column(
              crossAxisAlignment: .stretch,
              mainAxisSize: .min,
              children: [
                Text(
                  'Gain',
                  style: theme.typography.body.sm.copyWith(fontWeight: .w600),
                ),
                const SizedBox(height: 8),
                _GainRow(label: 'Loudness', value: _formatLufs(loudness)),
                _GainRow(
                  label: 'ReplayGain',
                  value: loudness == null || !loudness.isFinite
                      ? '—'
                      : _formatGainDb(-18 - loudness),
                ),
                _GainRow(label: 'Auto gain', value: _formatGainDb(autoGain)),
                _GainRow(label: 'Gain trim', value: _formatGainDb(trimDb)),
                _GainRow(label: 'Total gain', value: _formatGainDb(total)),
              ],
            ),
          ),
        );
      },
      builder: (context, controller, _) {
        final theme = context.theme;
        return GestureDetector(
          onTap: hasTrack ? controller.toggle : null,
          child: Semantics(
            button: true,
            enabled: hasTrack,
            label: 'Deck gain details',
            child: Padding(
              padding: const .all(4),
              child: Icon(
                LucideIcons.info,
                size: 14,
                color: hasTrack
                    ? theme.colors.mutedForeground
                    : theme.colors.border,
              ),
            ),
          ),
        );
      },
    );
  }
}

class _GainRow extends StatelessWidget {
  const _GainRow({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return Padding(
      padding: const .symmetric(vertical: 2),
      child: Row(
        children: [
          Expanded(
            child: Text(
              label,
              style: theme.typography.body.xs.copyWith(
                color: theme.colors.mutedForeground,
              ),
            ),
          ),
          Text(
            value,
            style: theme.typography.body.xs.copyWith(
              fontWeight: .w600,
              fontFeatures: const [FontFeature.tabularFigures()],
            ),
          ),
        ],
      ),
    );
  }
}

String _formatLufs(double? value) {
  if (value == null || !value.isFinite) {
    return '—';
  }
  return '${value.toStringAsFixed(1)} LUFS';
}

String _formatGainDb(double value) {
  final sign = value > 0 ? '+' : '';
  return '$sign${value.toStringAsFixed(1)} dB';
}
