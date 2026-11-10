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

/// (track, window origin, window span, picture width)
typedef StripWindowKey = (String, int, int, int);

/// (deck, window origin, window span, picture width)
typedef DeckStripWindowKey = (int, int, int, int);

class StripBeatGridPictureNotifier extends Notifier<Picture?> {
  new(this.arg);

  final StripWindowKey arg;
  Picture? _owned;

  @override
  Picture? build() {
    final (trackId, originMs, spanMs, widthPx) = arg;
    ref.onDispose(() {
      _dropPictureAfterFrame(_owned);
      _owned = null;
    });
    final grid = ref.watch(beatGridProvider(trackId));
    if (grid == null || grid.bpm == null || spanMs <= 0 || widthPx <= 0) {
      _dropPictureAfterFrame(_owned);
      _owned = null;
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
    final next = recordBeatGridPicture(
      marks: marks,
      phaseArrows: arrows,
      size: size,
    );
    _dropPictureAfterFrame(_owned);
    _owned = next;
    return next;
  }
}

class StripLoopPictureNotifier extends Notifier<Picture?> {
  new(this.arg);

  final StripWindowKey arg;
  Picture? _owned;

  @override
  Picture? build() {
    final (trackId, originMs, spanMs, widthPx) = arg;
    ref.onDispose(() {
      _dropPictureAfterFrame(_owned);
      _owned = null;
    });
    if (spanMs <= 0 || widthPx <= 0) {
      _dropPictureAfterFrame(_owned);
      _owned = null;
      return null;
    }
    final loops =
        ref.watch(trackSavedLoopsProvider.select((m) => m[trackId])) ??
        const [];
    if (loops.isEmpty) {
      _dropPictureAfterFrame(_owned);
      _owned = null;
      return null;
    }
    final next = recordLoopPicture(
      loops: loops,
      originMs: originMs,
      spanMs: spanMs,
      size: _stripSize(widthPx),
    );
    _dropPictureAfterFrame(_owned);
    _owned = next;
    return next;
  }
}

class StripActiveLoopPictureNotifier extends Notifier<Picture?> {
  new(this.arg);

  final DeckStripWindowKey arg;
  Picture? _owned;

  @override
  Picture? build() {
    final (deckId, originMs, spanMs, widthPx) = arg;
    ref.onDispose(() {
      _dropPictureAfterFrame(_owned);
      _owned = null;
    });
    if (spanMs <= 0 || widthPx <= 0) {
      _dropPictureAfterFrame(_owned);
      _owned = null;
      return null;
    }
    final loop = ref.watch(deckActiveLoopProvider(deckId));
    final next = recordActiveLoopPicture(
      loop: loop,
      originMs: originMs,
      spanMs: spanMs,
      size: _stripSize(widthPx),
    );
    _dropPictureAfterFrame(_owned);
    _owned = next;
    return next;
  }
}

class StripPendingLoopInPictureNotifier extends Notifier<Picture?> {
  new(this.arg);

  final DeckStripWindowKey arg;
  Picture? _owned;

  @override
  Picture? build() {
    final (deckId, originMs, spanMs, widthPx) = arg;
    ref.onDispose(() {
      _dropPictureAfterFrame(_owned);
      _owned = null;
    });
    if (spanMs <= 0 || widthPx <= 0) {
      _dropPictureAfterFrame(_owned);
      _owned = null;
      return null;
    }
    final pending = ref.watch(deckPendingLoopInMsProvider(deckId));
    final next = recordPendingLoopInPicture(
      pendingInMs: pending,
      originMs: originMs,
      spanMs: spanMs,
      size: _stripSize(widthPx),
    );
    _dropPictureAfterFrame(_owned);
    _owned = next;
    return next;
  }
}

class StripCuePictureNotifier extends Notifier<Picture?> {
  new(this.arg);

  final StripWindowKey arg;
  Picture? _owned;

  @override
  Picture? build() {
    final (trackId, originMs, spanMs, widthPx) = arg;
    ref.onDispose(() {
      _dropPictureAfterFrame(_owned);
      _owned = null;
    });
    if (spanMs <= 0 || widthPx <= 0) {
      _dropPictureAfterFrame(_owned);
      _owned = null;
      return null;
    }
    final rows = ref.watch(trackHotCuesProvider.select((m) => m[trackId]));
    if (rows == null || rows.isEmpty) {
      _dropPictureAfterFrame(_owned);
      _owned = null;
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
    final next = recordCuePicture(
      cues: cues,
      originMs: originMs,
      spanMs: spanMs,
      size: _stripSize(widthPx),
    );
    _dropPictureAfterFrame(_owned);
    _owned = next;
    return next;
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
