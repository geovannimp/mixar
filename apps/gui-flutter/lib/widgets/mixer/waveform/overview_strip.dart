import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/theme/mixar_colors.dart';
import 'package:gui_flutter/core/utils/waveform_picture.dart';
import 'package:gui_flutter/core/utils/waveform_section_painter.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/providers/mixer/waveform/waveform_providers.dart';
import 'package:gui_flutter/src/rust/api/event_mirrors.dart' show SavedLoop;
import 'package:gui_flutter/widgets/mixer/pads/hot_cue_pads.dart';
import 'package:gui_flutter/widgets/mixer/pads/pad_button.dart';
import 'package:gui_flutter/widgets/mixer/waveform/loop_band_delete.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:gui_flutter/widgets/shell/mixar_toast.dart';
import 'package:skeletonizer/skeletonizer.dart';

ShimmerEffect _skeletonEffect(MixarColors colors) => ShimmerEffect(
  baseColor: colors.waveformBackground,
  highlightColor: colors.border,
);

class OverviewStrip extends ConsumerWidget {
  const new({required this.deckId, this.height = 28, super.key});

  final int deckId;
  final double height;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = context.theme;
    final trackId = ref.watch(deckTrackIdProvider(deckId));
    final durationMs = ref.watch(deckDurationMsProvider(deckId)) ?? 0;
    final skeleton = ref.watch(deckSkeletonProvider(deckId));
    final texture = trackId == null
        ? null
        : ref.watch(waveformOverviewTextureProvider(trackId));
    final mode = ref.watch(waveformDisplayModeProvider);
    final hotCues = ref.watch(deckHotCuesProvider(deckId));
    final savedLoops = ref.watch(deckSavedLoopsProvider(deckId));
    final cuePointMs = ref.watch(deckCuePointMsProvider(deckId));

    return Skeletonizer(
      enabled: skeleton,
      effect: _skeletonEffect(theme.colors),
      child: Skeleton.replace(
        width: double.infinity,
        height: height,
        replacement: Skeleton.leaf(
          child: ColoredBox(color: theme.colors.waveformBackground),
        ),
        child: SizedBox(
          height: height,
          width: double.infinity,
          child: LayoutBuilder(
            builder: (context, constraints) {
              final width = constraints.maxWidth;
              return LoopBandDeleteRegion(
                deckId: deckId,
                trackId: trackId ?? '',
                loops: savedLoops,
                originMs: 0,
                spanMs: durationMs,
                width: width,
                height: height,
                compact: true,
                iconAbove: true,
                onTapMs: durationMs <= 0
                    ? null
                    : (ms) => unawaited(_seek(ref, context, deckId, ms)),
                child: ExcludeSemantics(
                  child: Stack(
                    fit: StackFit.expand,
                    children: [
                      CustomPaint(
                        painter: WaveformShaderPainter(
                          texture: texture,
                          background: theme.colors.waveformBackground,
                          mode: mode,
                        ),
                      ),
                      if (durationMs > 0 && width > 0)
                        IgnorePointer(
                          child: _OverviewOverlayLayer(
                            durationMs: durationMs,
                            width: width,
                            height: height,
                            hotCues: hotCues,
                            savedLoops: savedLoops,
                            cuePointMs: cuePointMs,
                            accent: theme.colors.primary,
                            cueColor: theme.colors.cueOn,
                          ),
                        ),
                      if (durationMs > 0)
                        RepaintBoundary(
                          child: _OverviewPlayhead(
                            deckId: deckId,
                            durationMs: durationMs,
                            width: width,
                            lineColor: theme.colors.foreground.withValues(
                              alpha: 0.85,
                            ),
                            shadeColor: _playedShade(theme.colors),
                          ),
                        ),
                    ],
                  ),
                ),
              );
            },
          ),
        ),
      ),
    );
  }
}

/// Veil over the played part of the overview.
///
/// A black scrim reads as "already played" on the dark lane; on the light lane
/// the same scrim would swallow the bars, so the wash comes from the
/// foreground colour at low alpha instead.
Color _playedShade(MixarColors colors) => colors.brightness == Brightness.dark
    ? const Color.fromRGBO(0, 0, 0, 0.6)
    : colors.foreground.withValues(alpha: 0.10);

/// Playhead shade + needle — watches position without rebuilding the waveform.
class _OverviewPlayhead extends ConsumerWidget {
  const new({
    required this.deckId,
    required this.durationMs,
    required this.width,
    required this.lineColor,
    required this.shadeColor,
  });

  final int deckId;
  final int durationMs;
  final double width;
  final Color lineColor;
  final Color shadeColor;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final positionMs = ref.watch(deckPositionMsProvider(deckId));
    final playheadX = durationMs > 0
        ? (positionMs / durationMs).clamp(0.0, 1.0) * width
        : 0.0;
    return Stack(
      fit: StackFit.expand,
      children: [
        Positioned(
          left: 0,
          width: playheadX,
          top: 0,
          bottom: 0,
          child: ColoredBox(color: shadeColor),
        ),
        Positioned(
          left: playheadX,
          top: 0,
          bottom: 0,
          child: ColoredBox(color: lineColor, child: const SizedBox(width: 1)),
        ),
      ],
    );
  }
}

/// Loop + cue sections for the overview (no beat grid / active loop).
class _OverviewOverlayLayer extends StatelessWidget {
  const new({
    required this.durationMs,
    required this.width,
    required this.height,
    required this.hotCues,
    required this.savedLoops,
    required this.cuePointMs,
    required this.accent,
    required this.cueColor,
  });

  final int durationMs;
  final double width;
  final double height;
  final List<DeckHotCue> hotCues;
  final List<SavedLoop> savedLoops;

  /// The deck's main cue, flagged apart from the numbered hot cues.
  final int? cuePointMs;
  final Color accent;
  final Color cueColor;

  @override
  Widget build(BuildContext context) {
    final sections = <WaveformSection>[
      for (final loop in savedLoops)
        WaveformSection(
          type: WaveformSectionType.loop,
          startMs: loop.inMs,
          endMs: loop.outMs,
          color: accent,
        ),
      if (cuePointMs case final cuePoint?)
        WaveformSection(
          type: WaveformSectionType.mainCue,
          startMs: cuePoint,
          color: cueColor,
        ),
      for (final cue in hotCues)
        WaveformSection(
          type: WaveformSectionType.cue,
          startMs: cue.positionMs,
          color: hotCueAccent(cue.slot).border.withValues(alpha: 1),
          label: '${cue.slot + 1}',
        ),
    ];
    return CustomPaint(
      size: Size(width, height),
      painter: WaveformSectionPainter(
        sections: sections,
        originMs: 0,
        spanMs: durationMs,
        compact: true,
      ),
    );
  }
}

Future<void> _seek(
  WidgetRef ref,
  BuildContext context,
  int deckId,
  int ms,
) async {
  try {
    await seekDeck(ref, deckId, ms);
  } catch (e) {
    if (!context.mounted) {
      return;
    }
    showMixarToast(
      context: context,
      variant: MixarToastVariant.destructive,
      title: Text('$e'),
    );
  }
}
