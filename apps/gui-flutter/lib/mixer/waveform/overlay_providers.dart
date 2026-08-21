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

void _dropPictureAfterFrame(Picture? picture) {
  if (picture == null) {
    return;
  }
  SchedulerBinding.instance.addPostFrameCallback((_) {
    picture.dispose();
  });
}

Size _stripSize(int durationMs) =>
    Size(stripWidthPx(durationMs).toDouble(), kWaveformStripHeight);

class StripBeatGridPictureNotifier extends Notifier<Picture?> {
  StripBeatGridPictureNotifier(this.arg);

  final (String, int) arg;
  Picture? _owned;

  @override
  Picture? build() {
    final (trackId, durationMs) = arg;
    ref.onDispose(() {
      _dropPictureAfterFrame(_owned);
      _owned = null;
    });
    final grid = ref.watch(beatGridProvider(trackId)).value;
    if (grid == null || grid.bpm == null || durationMs <= 0) {
      _dropPictureAfterFrame(_owned);
      _owned = null;
      return null;
    }
    final size = _stripSize(durationMs);
    final marks = beatGridXs(
      bpm: grid.bpm!,
      firstBeatSecs: grid.beats.isEmpty ? 0 : grid.beats.first,
      originMs: 0,
      spanMs: durationMs.toDouble(),
      width: size.width,
    );
    final next = recordBeatGridPicture(marks: marks, size: size);
    _dropPictureAfterFrame(_owned);
    _owned = next;
    return next;
  }
}

class StripLoopPictureNotifier extends Notifier<Picture?> {
  StripLoopPictureNotifier(this.arg);

  final (String, int) arg;
  Picture? _owned;

  @override
  Picture? build() {
    final (trackId, durationMs) = arg;
    ref.onDispose(() {
      _dropPictureAfterFrame(_owned);
      _owned = null;
    });
    if (durationMs <= 0) {
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
      durationMs: durationMs,
      size: _stripSize(durationMs),
    );
    _dropPictureAfterFrame(_owned);
    _owned = next;
    return next;
  }
}

class StripActiveLoopPictureNotifier extends Notifier<Picture?> {
  StripActiveLoopPictureNotifier(this.arg);

  final (int, int) arg;
  Picture? _owned;

  @override
  Picture? build() {
    final (deckId, durationMs) = arg;
    ref.onDispose(() {
      _dropPictureAfterFrame(_owned);
      _owned = null;
    });
    if (durationMs <= 0) {
      _dropPictureAfterFrame(_owned);
      _owned = null;
      return null;
    }
    final loop = ref.watch(deckActiveLoopProvider(deckId));
    final next = recordActiveLoopPicture(
      loop: loop,
      durationMs: durationMs,
      size: _stripSize(durationMs),
    );
    _dropPictureAfterFrame(_owned);
    _owned = next;
    return next;
  }
}

class StripCuePictureNotifier extends Notifier<Picture?> {
  StripCuePictureNotifier(this.arg);

  final (String, int) arg;
  Picture? _owned;

  @override
  Picture? build() {
    final (trackId, durationMs) = arg;
    ref.onDispose(() {
      _dropPictureAfterFrame(_owned);
      _owned = null;
    });
    if (durationMs <= 0) {
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
      durationMs: durationMs,
      size: _stripSize(durationMs),
    );
    _dropPictureAfterFrame(_owned);
    _owned = next;
    return next;
  }
}

final stripBeatGridPictureProvider =
    NotifierProvider.family<StripBeatGridPictureNotifier, Picture?, (String, int)>(
      StripBeatGridPictureNotifier.new,
    );

final stripLoopPictureProvider =
    NotifierProvider.family<StripLoopPictureNotifier, Picture?, (String, int)>(
      StripLoopPictureNotifier.new,
    );

final stripActiveLoopPictureProvider =
    NotifierProvider.family<StripActiveLoopPictureNotifier, Picture?, (int, int)>(
      StripActiveLoopPictureNotifier.new,
    );

final stripCuePictureProvider =
    NotifierProvider.family<StripCuePictureNotifier, Picture?, (String, int)>(
      StripCuePictureNotifier.new,
    );
