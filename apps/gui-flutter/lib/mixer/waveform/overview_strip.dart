import 'dart:async';
import 'dart:ui' as ui;

import 'package:flutter/scheduler.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/mixer/pads/hot_cue_pads.dart';
import 'package:gui_flutter/mixer/waveform/layout.dart';
import 'package:gui_flutter/mixer/waveform/overlay_pictures.dart';
import 'package:gui_flutter/mixer/waveform/peaks.dart';
import 'package:gui_flutter/mixer/waveform/spectral_color.dart';
import 'package:gui_flutter/mixer/waveform/waveform_picture.dart';
import 'package:gui_flutter/mixer/waveform/waveform_providers.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:skeletonizer/skeletonizer.dart';

const _waveformSkeletonEffect = ShimmerEffect(
  baseColor: kWaveformBg,
  highlightColor: Color.fromARGB(255, 32, 34, 42),
);

class OverviewStrip extends ConsumerWidget {
  const OverviewStrip({required this.deckId, this.height = 28, super.key});

  final int deckId;
  final double height;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = context.theme;
    final trackId = ref.watch(deckTrackIdProvider(deckId));
    final durationMs = ref.watch(deckDurationMsProvider(deckId)) ?? 0;
    final positionMs = ref.watch(deckPositionMsProvider(deckId));
    final skeleton = ref.watch(deckSkeletonProvider(deckId));
    final peaks = trackId == null
        ? const <SpectralPeak>[]
        : (ref.watch(waveformOverviewProvider(trackId)).value ??
              const <SpectralPeak>[]);
    final mode = ref.watch(waveformDisplayModeProvider);
    final hotCues = ref.watch(deckHotCuesProvider(deckId));
    final savedLoops = ref.watch(deckSavedLoopsProvider(deckId));

    return Skeletonizer(
      enabled: skeleton,
      effect: _waveformSkeletonEffect,
      child: Skeleton.replace(
        width: double.infinity,
        height: height,
        replacement: const Skeleton.leaf(child: ColoredBox(color: kWaveformBg)),
        child: SizedBox(
          height: height,
          width: double.infinity,
          child: LayoutBuilder(
            builder: (context, constraints) {
              final width = constraints.maxWidth;
              final visibleMs = ref.watch(waveformVisibleMsProvider);
              final window = overviewWindowRect(
                positionMs: positionMs,
                durationMs: durationMs,
                visibleMs: clampWaveformVisibleMs(
                  visibleMs,
                  durationMs: durationMs,
                ),
              );
              return GestureDetector(
                behavior: HitTestBehavior.opaque,
                onTapDown: durationMs <= 0
                    ? null
                    : (details) {
                        final ms =
                            (details.localPosition.dx / width * durationMs)
                                .round();
                        unawaited(_seek(ref, context, deckId, ms));
                      },
                child: Stack(
                  fit: StackFit.expand,
                  children: [
                    CustomPaint(
                      painter: WaveformBarPainter(
                        overview: peaks,
                        detail: null,
                        durationMs: durationMs,
                        originMs: 0,
                        spanMs: durationMs.toDouble(),
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
                        ),
                      ),
                    if (durationMs > 0)
                      Positioned(
                        left: window.left * width,
                        width:
                            (window.right - window.left).clamp(0.0, 1.0) *
                            width,
                        top: 0,
                        bottom: 0,
                        child: ColoredBox(
                          color: theme.colors.foreground.withValues(
                            alpha: 0.12,
                          ),
                        ),
                      ),
                    if (durationMs > 0)
                      Positioned(
                        left: (positionMs / durationMs).clamp(0.0, 1.0) * width,
                        top: 0,
                        bottom: 0,
                        child: ColoredBox(
                          color: theme.colors.foreground.withValues(
                            alpha: 0.85,
                          ),
                          child: const SizedBox(width: 1),
                        ),
                      ),
                  ],
                ),
              );
            },
          ),
        ),
      ),
    );
  }
}

/// Cached Loop + Cue pictures for the overview (no beat grid / active loop).
class _OverviewOverlayLayer extends StatefulWidget {
  const _OverviewOverlayLayer({
    required this.durationMs,
    required this.width,
    required this.height,
    required this.hotCues,
    required this.savedLoops,
  });

  final int durationMs;
  final double width;
  final double height;
  final List<DeckHotCue> hotCues;
  final List<SavedLoopInfo> savedLoops;

  @override
  State<_OverviewOverlayLayer> createState() => _OverviewOverlayLayerState();
}

class _OverviewOverlayLayerState extends State<_OverviewOverlayLayer> {
  ui.Picture? _loops;
  ui.Picture? _cues;
  int? _loopKey;
  int? _cueKey;

  @override
  void didUpdateWidget(covariant _OverviewOverlayLayer oldWidget) {
    super.didUpdateWidget(oldWidget);
    _syncPictures();
  }

  @override
  void initState() {
    super.initState();
    _syncPictures();
  }

  @override
  void dispose() {
    _drop(_loops);
    _drop(_cues);
    super.dispose();
  }

  void _drop(ui.Picture? picture) {
    if (picture == null) {
      return;
    }
    SchedulerBinding.instance.addPostFrameCallback((_) {
      picture.dispose();
    });
  }

  void _syncPictures() {
    final size = Size(widget.width, widget.height);
    final loopKey = Object.hash(
      widget.durationMs,
      widget.width,
      widget.height,
      Object.hashAll(widget.savedLoops),
    );
    if (loopKey != _loopKey) {
      _drop(_loops);
      _loops = widget.savedLoops.isEmpty
          ? null
          : recordLoopPicture(
              loops: widget.savedLoops,
              durationMs: widget.durationMs,
              size: size,
            );
      _loopKey = loopKey;
    }
    final cueKey = Object.hash(
      widget.durationMs,
      widget.width,
      widget.height,
      Object.hashAll(widget.hotCues.map((c) => Object.hash(c.slot, c.positionMs))),
    );
    if (cueKey != _cueKey) {
      _drop(_cues);
      _cues = widget.hotCues.isEmpty
          ? null
          : recordCuePicture(
              cues: widget.hotCues,
              durationMs: widget.durationMs,
              size: size,
            );
      _cueKey = cueKey;
    }
  }

  @override
  Widget build(BuildContext context) {
    return CustomPaint(
      painter: _OverviewOverlayPainter(loops: _loops, cues: _cues),
      size: Size(widget.width, widget.height),
    );
  }
}

class _OverviewOverlayPainter extends CustomPainter {
  _OverviewOverlayPainter({required this.loops, required this.cues});

  final ui.Picture? loops;
  final ui.Picture? cues;

  @override
  void paint(Canvas canvas, Size size) {
    if (loops != null) {
      canvas.drawPicture(loops!);
    }
    if (cues != null) {
      canvas.drawPicture(cues!);
    }
  }

  @override
  bool shouldRepaint(_OverviewOverlayPainter oldDelegate) =>
      !identical(loops, oldDelegate.loops) || !identical(cues, oldDelegate.cues);
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
    showFToast(context: context, variant: .destructive, title: Text('$e'));
  }
}
