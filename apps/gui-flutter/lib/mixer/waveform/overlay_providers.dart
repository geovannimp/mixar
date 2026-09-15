import 'dart:ui';

import 'package:flutter/scheduler.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/mixer/pads/hot_cue_pads.dart';
import 'package:gui_flutter/mixer/waveform/beat_grid.dart';
import 'package:gui_flutter/mixer/waveform/layout.dart';
import 'package:gui_flutter/mixer/waveform/overlay_pictures.dart';
import 'package:gui_flutter/mixer/waveform/waveform_providers.dart';
import 'package:gui_flutter/mixer/waveform/waveform_ring.dart';

void _dropPictureAfterFrame(Picture? picture) {
  if (picture == null) {
    return;
  }
  SchedulerBinding.instance.addPostFrameCallback((_) {
    picture.dispose();
  });
}

Size _ringSize(WaveformRing ring) =>
    Size(ring.widthPx.toDouble(), kWaveformStripHeight);

class RingBeatGridPictureNotifier extends Notifier<Picture?> {
  RingBeatGridPictureNotifier(this.arg);

  final (int, int, int) arg;
  Picture? _owned;

  @override
  Picture? build() {
    ref.onDispose(() {
      _dropPictureAfterFrame(_owned);
      _owned = null;
    });
    final ring = ref.watch(waveformRingProvider(arg));
    final trackId = ref.watch(deckTrackIdProvider(arg.$1));
    if (ring == null || trackId == null) {
      _dropPictureAfterFrame(_owned);
      _owned = null;
      return null;
    }
    final grid = ref.watch(beatGridProvider(trackId));
    if (grid == null || grid.bpm == null) {
      _dropPictureAfterFrame(_owned);
      _owned = null;
      return null;
    }
    final size = _ringSize(ring);
    final marks = beatGridXs(
      bpm: grid.bpm!,
      firstBeatSecs: grid.beats.isEmpty ? 0 : grid.beats.first,
      originMs: ring.originMs,
      spanMs: ring.spanMs,
      width: size.width,
    );
    final next = recordBeatGridPicture(marks: marks, size: size);
    _dropPictureAfterFrame(_owned);
    _owned = next;
    return next;
  }
}

class RingLoopPictureNotifier extends Notifier<Picture?> {
  RingLoopPictureNotifier(this.arg);

  final (int, int, int) arg;
  Picture? _owned;

  @override
  Picture? build() {
    ref.onDispose(() {
      _dropPictureAfterFrame(_owned);
      _owned = null;
    });
    final ring = ref.watch(waveformRingProvider(arg));
    final trackId = ref.watch(deckTrackIdProvider(arg.$1));
    if (ring == null || trackId == null) {
      _dropPictureAfterFrame(_owned);
      _owned = null;
      return null;
    }
    final loops = ref.watch(trackSavedLoopsProvider)[trackId] ?? const [];
    if (loops.isEmpty) {
      _dropPictureAfterFrame(_owned);
      _owned = null;
      return null;
    }
    final next = recordLoopPicture(
      loops: loops,
      originMs: ring.originMs,
      spanMs: ring.spanMs,
      size: _ringSize(ring),
    );
    _dropPictureAfterFrame(_owned);
    _owned = next;
    return next;
  }
}

class RingActiveLoopPictureNotifier extends Notifier<Picture?> {
  RingActiveLoopPictureNotifier(this.arg);

  final (int, int, int) arg;
  Picture? _owned;

  @override
  Picture? build() {
    ref.onDispose(() {
      _dropPictureAfterFrame(_owned);
      _owned = null;
    });
    final ring = ref.watch(waveformRingProvider(arg));
    if (ring == null) {
      _dropPictureAfterFrame(_owned);
      _owned = null;
      return null;
    }
    final loop = ref.watch(deckActiveLoopProvider(arg.$1));
    final next = recordActiveLoopPicture(
      loop: loop,
      originMs: ring.originMs,
      spanMs: ring.spanMs,
      size: _ringSize(ring),
    );
    _dropPictureAfterFrame(_owned);
    _owned = next;
    return next;
  }
}

class RingPendingLoopInPictureNotifier extends Notifier<Picture?> {
  RingPendingLoopInPictureNotifier(this.arg);

  final (int, int, int) arg;
  Picture? _owned;

  @override
  Picture? build() {
    ref.onDispose(() {
      _dropPictureAfterFrame(_owned);
      _owned = null;
    });
    final ring = ref.watch(waveformRingProvider(arg));
    if (ring == null) {
      _dropPictureAfterFrame(_owned);
      _owned = null;
      return null;
    }
    final pending = ref.watch(deckPendingLoopInMsProvider(arg.$1));
    final next = recordPendingLoopInPicture(
      pendingInMs: pending,
      originMs: ring.originMs,
      spanMs: ring.spanMs,
      size: _ringSize(ring),
    );
    _dropPictureAfterFrame(_owned);
    _owned = next;
    return next;
  }
}

class RingCuePictureNotifier extends Notifier<Picture?> {
  RingCuePictureNotifier(this.arg);

  final (int, int, int) arg;
  Picture? _owned;

  @override
  Picture? build() {
    ref.onDispose(() {
      _dropPictureAfterFrame(_owned);
      _owned = null;
    });
    final ring = ref.watch(waveformRingProvider(arg));
    final trackId = ref.watch(deckTrackIdProvider(arg.$1));
    if (ring == null || trackId == null) {
      _dropPictureAfterFrame(_owned);
      _owned = null;
      return null;
    }
    final rows = ref.watch(trackHotCuesProvider)[trackId];
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
      originMs: ring.originMs,
      spanMs: ring.spanMs,
      size: _ringSize(ring),
    );
    _dropPictureAfterFrame(_owned);
    _owned = next;
    return next;
  }
}

final ringBeatGridPictureProvider = NotifierProvider.autoDispose
    .family<RingBeatGridPictureNotifier, Picture?, (int, int, int)>(
      RingBeatGridPictureNotifier.new,
    );

final ringLoopPictureProvider = NotifierProvider.autoDispose
    .family<RingLoopPictureNotifier, Picture?, (int, int, int)>(
      RingLoopPictureNotifier.new,
    );

final ringActiveLoopPictureProvider = NotifierProvider.autoDispose
    .family<RingActiveLoopPictureNotifier, Picture?, (int, int, int)>(
      RingActiveLoopPictureNotifier.new,
    );

final ringPendingLoopInPictureProvider = NotifierProvider.autoDispose
    .family<RingPendingLoopInPictureNotifier, Picture?, (int, int, int)>(
      RingPendingLoopInPictureNotifier.new,
    );

final ringCuePictureProvider = NotifierProvider.autoDispose
    .family<RingCuePictureNotifier, Picture?, (int, int, int)>(
      RingCuePictureNotifier.new,
    );
