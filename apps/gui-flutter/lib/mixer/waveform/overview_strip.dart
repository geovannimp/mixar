import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/mixer/waveform/layout.dart';
import 'package:gui_flutter/mixer/waveform/peaks.dart';
import 'package:gui_flutter/mixer/waveform/waveform_picture.dart';
import 'package:gui_flutter/mixer/waveform/waveform_providers.dart';

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
    final speed = ref.watch(deckSpeedRatioProvider(deckId));
    final peaks = trackId == null
        ? const <SpectralPeak>[]
        : (ref.watch(waveformOverviewProvider(trackId)).value ??
              const <SpectralPeak>[]);
    final visibleMs = visibleSourceMs(speed);
    final window = overviewWindowRect(
      positionMs: positionMs,
      durationMs: durationMs,
      visibleMs: visibleMs,
    );

    return SizedBox(
      height: height,
      width: double.infinity,
      child: LayoutBuilder(
        builder: (context, constraints) {
          final width = constraints.maxWidth;
          return GestureDetector(
            behavior: HitTestBehavior.opaque,
            onTapDown: durationMs <= 0
                ? null
                : (details) {
                    final ms = (details.localPosition.dx / width * durationMs)
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
                  ),
                ),
                if (durationMs > 0)
                  Positioned(
                    left: window.left * width,
                    width: (window.right - window.left).clamp(0.0, 1.0) * width,
                    top: 0,
                    bottom: 0,
                    child: ColoredBox(
                      color: theme.colors.foreground.withValues(alpha: 0.12),
                    ),
                  ),
                if (durationMs > 0)
                  Positioned(
                    left: (positionMs / durationMs).clamp(0.0, 1.0) * width,
                    top: 0,
                    bottom: 0,
                    child: ColoredBox(
                      color: theme.colors.foreground.withValues(alpha: 0.85),
                      child: const SizedBox(width: 1),
                    ),
                  ),
              ],
            ),
          );
        },
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
    showFToast(context: context, variant: .destructive, title: Text('$e'));
  }
}
