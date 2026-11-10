import 'dart:ui';

import 'package:flutter/scheduler.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/utils/waveform_beat_grid.dart';
import 'package:gui_flutter/core/utils/waveform_layout.dart';
import 'package:gui_flutter/core/utils/waveform_overlay_pictures.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/providers/mixer/waveform/waveform_providers.dart';
import 'package:gui_flutter/widgets/mixer/pads/hot_cue_pads.dart';
import 'package:riverpod/src/providers/notifier.dart';

void _dropPictureAfterFrame(Picture? picture) {
  if (picture == null) {
    return;
  }
  SchedulerBinding.instance.addPostFrameCallback((_) {
    picture.dispose();
  });
}

/// Overlays live in the same source-time window as the strip, so they re-record
/// (cheaply) whenever the lane recentres or zooms; peaks are not involved.
Size _stripSize(int widthPx) => Size(widthPx.toDouble(), kWaveformStripHeight);

/// Picture ownership shared by every strip overlay notifier.
///
/// Each overlay drops, guards and replaces its picture the same way; keeping
/// that in one place stops the teardown from drifting between them.
mixin _OverlayPicture on Notifier<Picture?> {
  Picture? _picture;

  /// Releases the picture after the frame that may still paint it.
  void dropPicture() {
    _dropPictureAfterFrame(_picture);
    _picture = null;
  }

  /// Drops the picture and reports whether the window cannot be painted.
  bool dropInvalidWindow(int spanMs, int widthPx) {
    if (spanMs > 0 && widthPx > 0) {
      return false;
    }
    dropPicture();
    return true;
  }

  /// Replaces the owned picture and returns it for the notifier's state.
  Picture? publishPicture(Picture? next) {
    _dropPictureAfterFrame(_picture);
    _picture = next;
    return next;
  }
}

/// (track, window origin, window span, picture width)
typedef StripWindowKey = (String, int, int, int);

/// (deck, window origin, window span, picture width)
typedef DeckStripWindowKey = (int, int, int, int);

class StripBeatGridPictureNotifier extends Notifier<Picture?>
    with _OverlayPicture {
  new(this.arg);

  final StripWindowKey arg;

  @override
  Picture? build() {
    final (trackId, originMs, spanMs, widthPx) = arg;
    ref.onDispose(dropPicture);
    final grid = ref.watch(beatGridProvider(trackId));
    if (grid == null || grid.bpm == null) {
      dropPicture();
      return null;
    }
    if (dropInvalidWindow(spanMs, widthPx)) {
      return null;
    }
    final size = _stripSize(widthPx);
    final marks = beatGridXs(
      bpm: grid.bpm!,
      firstBeatSecs: grid.beats.isEmpty ? 0 : grid.beats.first,
      originMs: originMs.toDouble(),
      spanMs: spanMs.toDouble(),
      width: size.width,
    );
    final arrows = phaseMarkXs(
      bpm: grid.bpm,
      downbeatsSecs: grid.downbeats,
      firstBeatSecs: grid.beats.isEmpty ? 0 : grid.beats.first,
      originMs: originMs.toDouble(),
      spanMs: spanMs.toDouble(),
      width: size.width,
    );
    return publishPicture(
      recordBeatGridPicture(marks: marks, phaseArrows: arrows, size: size),
    );
  }
}

class StripLoopPictureNotifier extends Notifier<Picture?> with _OverlayPicture {
  new(this.arg);

  final StripWindowKey arg;

  @override
  Picture? build() {
    final (trackId, originMs, spanMs, widthPx) = arg;
    ref.onDispose(dropPicture);
    if (dropInvalidWindow(spanMs, widthPx)) {
      return null;
    }
    final loops =
        ref.watch(trackSavedLoopsProvider.select((m) => m[trackId])) ??
        const [];
    if (loops.isEmpty) {
      dropPicture();
      return null;
    }
    return publishPicture(
      recordLoopPicture(
        loops: loops,
        originMs: originMs,
        spanMs: spanMs,
        size: _stripSize(widthPx),
      ),
    );
  }
}

class StripActiveLoopPictureNotifier extends Notifier<Picture?>
    with _OverlayPicture {
  new(this.arg);

  final DeckStripWindowKey arg;

  @override
  Picture? build() {
    final (deckId, originMs, spanMs, widthPx) = arg;
    ref.onDispose(dropPicture);
    if (dropInvalidWindow(spanMs, widthPx)) {
      return null;
    }
    return publishPicture(
      recordActiveLoopPicture(
        loop: ref.watch(deckActiveLoopProvider(deckId)),
        originMs: originMs,
        spanMs: spanMs,
        size: _stripSize(widthPx),
      ),
    );
  }
}

class StripPendingLoopInPictureNotifier extends Notifier<Picture?>
    with _OverlayPicture {
  new(this.arg);

  final DeckStripWindowKey arg;

  @override
  Picture? build() {
    final (deckId, originMs, spanMs, widthPx) = arg;
    ref.onDispose(dropPicture);
    if (dropInvalidWindow(spanMs, widthPx)) {
      return null;
    }
    return publishPicture(
      recordPendingLoopInPicture(
        pendingInMs: ref.watch(deckPendingLoopInMsProvider(deckId)),
        originMs: originMs,
        spanMs: spanMs,
        size: _stripSize(widthPx),
      ),
    );
  }
}

class StripCuePictureNotifier extends Notifier<Picture?> with _OverlayPicture {
  new(this.arg);

  final StripWindowKey arg;

  @override
  Picture? build() {
    final (trackId, originMs, spanMs, widthPx) = arg;
    ref.onDispose(dropPicture);
    if (dropInvalidWindow(spanMs, widthPx)) {
      return null;
    }
    final rows = ref.watch(trackHotCuesProvider.select((m) => m[trackId]));
    if (rows == null || rows.isEmpty) {
      dropPicture();
      return null;
    }
    final cues = [
      for (final row in rows)
        DeckHotCue(
          slot: row.slot,
          positionMs: row.positionMs,
          label: row.label,
        ),
    ];
    return publishPicture(
      recordCuePicture(
        cues: cues,
        originMs: originMs,
        spanMs: spanMs,
        size: _stripSize(widthPx),
      ),
    );
  }
}

final NotifierProviderFamily<
  StripBeatGridPictureNotifier,
  Picture?,
  StripWindowKey
>
stripBeatGridPictureProvider = NotifierProvider.autoDispose
    .family<StripBeatGridPictureNotifier, Picture?, StripWindowKey>(
      StripBeatGridPictureNotifier.new,
    );

final NotifierProviderFamily<StripLoopPictureNotifier, Picture?, StripWindowKey>
stripLoopPictureProvider = NotifierProvider.autoDispose
    .family<StripLoopPictureNotifier, Picture?, StripWindowKey>(
      StripLoopPictureNotifier.new,
    );

final NotifierProviderFamily<
  StripActiveLoopPictureNotifier,
  Picture?,
  DeckStripWindowKey
>
stripActiveLoopPictureProvider = NotifierProvider.autoDispose
    .family<StripActiveLoopPictureNotifier, Picture?, DeckStripWindowKey>(
      StripActiveLoopPictureNotifier.new,
    );

final NotifierProviderFamily<
  StripPendingLoopInPictureNotifier,
  Picture?,
  DeckStripWindowKey
>
stripPendingLoopInPictureProvider = NotifierProvider.autoDispose
    .family<StripPendingLoopInPictureNotifier, Picture?, DeckStripWindowKey>(
      StripPendingLoopInPictureNotifier.new,
    );

final NotifierProviderFamily<StripCuePictureNotifier, Picture?, StripWindowKey>
stripCuePictureProvider = NotifierProvider.autoDispose
    .family<StripCuePictureNotifier, Picture?, StripWindowKey>(
      StripCuePictureNotifier.new,
    );
