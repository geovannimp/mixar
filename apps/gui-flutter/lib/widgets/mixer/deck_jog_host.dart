import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/widgets/mixer/fader_slider.dart';
import 'package:gui_flutter/widgets/shell/mixar_toast.dart';
import 'package:gui_flutter/widgets/mixer/jog_platter.dart';

/// Interactive jog platter — `jog_touch` / `jog_turn` (Tauri `JogPlatter`).
class DeckJogHost extends ConsumerWidget {
  const new({
    required this.deckId,
    required this.hasTrack,
    required this.accent,
    this.disabled = false,
    super.key,
  });

  final int deckId;
  final bool hasTrack;
  final FaderAccent accent;
  final bool disabled;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return JogPlatter(
      accent: accent,
      playing: ref.watch(deckPlayingProvider(deckId)),
      bpm: ref.watch(deckBpmProvider(deckId)),
      hasTrack: hasTrack,
      enabled: !disabled && hasTrack,
      jogTouching: ref.watch(deckJogTouchingProvider(deckId)),
      positionMs: ref.watch(deckPositionMsProvider(deckId)),
      durationMs: ref.watch(deckDurationMsProvider(deckId)),
      speed: ref.watch(deckSpeedRatioProvider(deckId)),
      onJogTouch: (touching) {
        unawaited(_engineCmd(context, () => jogTouch(ref, deckId, touching)));
      },
      onJogTurn: (delta) {
        unawaited(_engineCmd(context, () => jogTurn(ref, deckId, delta)));
      },
    );
  }
}

Future<void> _engineCmd(
  BuildContext context,
  Future<void> Function() fn,
) async {
  try {
    await fn();
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
