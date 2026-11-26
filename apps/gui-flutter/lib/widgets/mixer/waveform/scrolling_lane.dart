import 'dart:ui';

import 'package:flutter/gestures.dart' show PointerScrollEvent, kPrimaryButton;
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/models/pad_modes.dart';
import 'package:gui_flutter/core/theme/app_typography.dart';
import 'package:gui_flutter/core/theme/deck_accent.dart';
import 'package:gui_flutter/core/utils/jog_ticks.dart';
import 'package:gui_flutter/core/utils/waveform_gain.dart';
import 'package:gui_flutter/core/utils/waveform_layout.dart';
import 'package:gui_flutter/core/utils/waveform_picture.dart';
import 'package:gui_flutter/core/utils/waveform_section_painter.dart';
import 'package:gui_flutter/core/utils/waveform_spectral_color.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/providers/mixer/waveform/overlay_providers.dart';
import 'package:gui_flutter/providers/mixer/waveform/waveform_providers.dart';
import 'package:gui_flutter/providers/mixer/waveform/waveform_strip.dart';
import 'package:gui_flutter/providers/mixer/waveform/waveform_zoom.dart';
import 'package:gui_flutter/src/rust/api/engine.dart' show EngineTransport;
import 'package:gui_flutter/widgets/mixer/pads/pad_button.dart';
import 'package:gui_flutter/widgets/mixer/track_end_blink.dart';
import 'package:gui_flutter/widgets/mixer/waveform/loop_band_delete.dart';
import 'package:gui_flutter/widgets/mixer/waveform/waveform_frame_clock.dart';
import 'package:gui_flutter/widgets/mixer/waveform/waveform_scroll_viewport.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:gui_flutter/widgets/shell/mixar_toast.dart';

class ScrollingLane extends ConsumerStatefulWidget {
  const new({required this.deckId, this.label, super.key});

  final int deckId;

  /// Deck letter painted in the lane's leading corner. Vertical lanes leave it
  /// null: the text would be rotated with the lane and unreadable.
  final String? label;

  @override
  ConsumerState<ScrollingLane> createState() => _ScrollingLaneState();
}

class _ScrollingLaneState extends ConsumerState<ScrollingLane> {
  /// Frame clock shared with every other lane, so both evaluate at the same
  /// timestamp instead of measuring elapsed from their own controller start.
  final WaveformFrameClock _clock = waveformFrameClock;
  var _attached = false;

  /// Marks the paint side dirty for *authoritative* position changes, which can
  /// happen while the clock is detached (paused, vinyl, seek). A counter rather
  /// than a bare [ChangeNotifier] because `notifyListeners` is protected.
  final _anchorTicks = ValueNotifier<int>(0);

  /// Source position shown at [_anchorFrame], and the rate it advances at.
  var _anchorMs = 0.0;
  var _anchorFrame = Duration.zero;
  var _speed = 1.0;
  var _playing = false;

  /// Drift correction, interpolated against the same frame clock.
  final _drift = DriftGlide();

  /// Playhead ticks plus out-of-track moves, so the strip follows a position
  /// dragged before the start or past the end as well as normal playback.
  late final Listenable _displayTicks;

  /// Signed source position while the playhead sits outside `[0, duration]`.
  /// The controller is clamped to the track, so it cannot represent those
  /// positions on its own.
  final ValueNotifier<double?> _outsideMs = ValueNotifier<double?>(null);

  var _scrubbing = false;

  /// Pointer that owns the current scratch drag, null when none is active.
  int? _scrubPointer;
  var _scrubAnchorX = 0.0;
  var _scrubAnchorMs = 0.0;

  /// Last preview target that was finite, so a release with non-finite
  /// coordinates still lands somewhere the lane was already showing.
  var _lastScrubMs = 0.0;

  /// Cumulative jog ticks already queued for the current press. Absolute, so
  /// sub-tick moves and reversals keep their precise net difference.
  var _lastQueuedTicks = 0;

  /// Platter touch state captured on the press's first nonzero tick.
  _WaveformScratch? _scratch;

  /// One serial command tail per lane: touch-on, turns and the release seek go
  /// out in submission order, and cleanup still works after unmount.
  Future<void> _scrubQueue = Future<void>.value();

  /// Bumped whenever the target track can no longer be trusted, so a queued
  /// seek that has not been published yet is dropped.
  int _seekEpoch = 0;

  var _disposed = false;

  @override
  void initState() {
    super.initState();
    _displayTicks = Listenable.merge([_clock, _outsideMs, _anchorTicks]);
    _clock.addListener(_onClockTick);
  }

  @override
  void didUpdateWidget(ScrollingLane oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.deckId != widget.deckId) {
      _seekEpoch++;
      _finishScrub();
    }
  }

  @override
  void dispose() {
    _disposed = true;
    _seekEpoch++;
    _finishScrub();
    _clock.removeListener(_onClockTick);
    if (_attached) {
      _clock.detach();
      _attached = false;
    }
    _anchorTicks.dispose();
    _outsideMs.dispose();
    super.dispose();
  }

  /// Keeps the shared clock running only while this lane needs frames.
  void _syncClockAttachment() {
    final need = _playing || _drift.pendingAt(_clock.now);
    if (need && !_attached) {
      _clock.attach();
      _attached = true;
    } else if (!need && _attached) {
      _clock.detach();
      _attached = false;
    }
  }

  void _onClockTick() => _syncClockAttachment();

  double _displayMs(int durationMs) {
    final outside = _outsideMs.value;
    if (outside != null) {
      return outside;
    }
    if (durationMs <= 0) {
      return 0;
    }
    final now = _clock.now;
    final base = _playing
        ? _anchorMs + (now - _anchorFrame).inMicroseconds / 1000.0 * _speed
        : _anchorMs;
    final ms = base + _drift.valueAt(now);
    return ms.clamp(0.0, durationMs.toDouble());
  }

  /// Shows [ms] on the lane, including positions outside the track.
  ///
  /// The controller only spans `0..duration`; the signed override keeps the
  /// strip moving when the playhead is dragged before the start or past the
  /// end, where the controller value would otherwise clamp and freeze.
  ///
  /// Authoritative: any pending drift correction is folded away first, so a
  /// seek, pause or scrub never drags a leftover offset into the new position.
  void _showMs(double ms, {required int durationMs}) {
    // An authoritative position ends any error streak.
    _drift.reset();
    _anchorMs = ms;
    _anchorFrame = _clock.now;
    if (durationMs <= 0) {
      _outsideMs.value = null;
      // Nothing else moves the strip here: the clock may be detached, so the
      // lane has to mark its own paint side dirty.
      _anchorTicks.value++;
      _syncClockAttachment();
      return;
    }
    // Set after the anchor so a clamped in-range position cannot publish over
    // the signed override.
    _outsideMs.value = (ms < 0 || ms > durationMs) ? ms : null;
    _anchorTicks.value++;
    // Keep the attachment invariant true at the call site instead of relying
    // on the next tick to notice the pending correction was dropped.
    _syncClockAttachment();
  }

  void _setDisplayMs(
    double ms, {
    required int durationMs,
    required double speed,
    required bool playing,
  }) {
    _showMs(ms, durationMs: durationMs);
    _speed = speed;
    if (durationMs <= 0) {
      _playing = false;
      _syncClockAttachment();
      return;
    }
    final outside = ms < 0 || ms > durationMs;
    _playing = playing && !_scrubbing && !outside;
    _syncClockAttachment();
  }

  /// Glides the displayed playhead toward the engine estimate.
  ///
  /// Only the offset animates: the playback anchor keeps its value and rate,
  /// running simulation, so the strip neither jumps now nor loses the
  /// interpolation it is already showing.
  void _correctDrift({
    required double engineMs,
    required int durationMs,
    required double speed,
  }) {
    // A paused lane keeps its anchor frozen but still adds the drift offset to
    // the displayed position, so scheduling a correction here would move a
    // stopped strip. Position events can arrive in either order relative to the
    // playing event, so gate on the state _displayMs actually uses.
    if (!_playing) {
      return;
    }
    final display = _displayMs(durationMs);
    final corrected = correctPlayheadDrift(
      displayMs: display,
      estimateMs: engineMs,
    );
    if (corrected == display) {
      return;
    }
    final delta = corrected - display;
    if (delta.abs() < 0.01) {
      return;
    }
    // Retargeting starts from the offset's current value, so an unfinished
    // correction keeps its progress instead of restarting from zero. The
    // default (linear) curve holds the pull at a constant rate, and
    // [playheadDriftDuration] keeps that rate within a quarter of the deck's
    // own source velocity, so a correction can never run the strip backwards
    // however slowly the deck plays.
    final now = _clock.now;
    _drift.retarget(
      at: now,
      target: _drift.valueAt(now) + delta,
      glide: playheadDriftDuration(deltaMs: delta, speed: speed),
    );
    _syncClockAttachment();
  }

  void _syncPlayback({
    required bool playing,
    required int durationMs,
    required double speed,
  }) {
    final now = _clock.now;
    if (durationMs <= 0) {
      _outsideMs.value = null;
      _drift.reset();
      _anchorMs = 0;
      _anchorFrame = now;
      _playing = false;
      _speed = speed;
      _anchorTicks.value++;
      _syncClockAttachment();
      return;
    }
    final outside = _outsideMs.value != null;
    final advancingNow = playing && !_scrubbing && !outside;
    // Re-anchor on every stop, start or rate change so the displayed position
    // is continuous across it; the anchor is what makes that true.
    final speedChanged = (speed - _speed).abs() > 1e-9;
    if (!advancingNow || speedChanged || !_playing) {
      final display = _displayMs(durationMs);
      _anchorMs = display;
      _anchorFrame = now;
      _speed = speed;
      // Absorb the pending correction into the anchor instead of letting it
      // glide on. Its velocity was sized for the previous speed, and a pull of
      // 0.25 x the old speed can exceed 25% of a slower deck's velocity, or
      // even outweigh it and run the strip backwards. The residual error is
      // re-detected on the next poll and corrected within the new bound.
      _drift.reset();
    }
    _playing = advancingNow;
    _syncClockAttachment();
  }

  bool _advancingNow() => playheadAdvancing(
    playing: ref.read(deckPlayingProvider(widget.deckId)),
    jogTouching: ref.read(deckJogTouchingProvider(widget.deckId)),
  );

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final advancing = playheadAdvancing(
      playing: ref.watch(deckPlayingProvider(widget.deckId)),
      jogTouching: ref.watch(deckJogTouchingProvider(widget.deckId)),
    );
    final speed = ref.watch(deckSpeedRatioProvider(widget.deckId));
    final trackId = ref.watch(deckTrackIdProvider(widget.deckId));
    final durationMs = ref.watch(deckDurationMsProvider(widget.deckId)) ?? 0;
    final slipOn = ref.watch(deckSlipEnabledProvider(widget.deckId));
    final slipShadowMs = ref.watch(deckSlipShadowMsProvider(widget.deckId));
    final zoomMs = ref.watch(waveformZoomProvider);

    // Do not watch deckPositionMsProvider here — ~30Hz Position ticks would
    // rebuild LayoutBuilder/Listener/Stack. Seed pause/vinyl on play/jog
    // rebuilds; ongoing ticks drive _playhead via ref.listen below.
    if (!advancing && !_scrubbing && durationMs > 0) {
      final enginePosMs = ref.read(deckPositionMsProvider(widget.deckId));
      if ((_displayMs(durationMs) - enginePosMs).abs() > 0.5) {
        _showMs(enginePosMs.toDouble(), durationMs: durationMs);
      }
    }

    _syncPlayback(playing: advancing, durationMs: durationMs, speed: speed);

    // Pause/vinyl always snaps; advancing only snaps large seeks / corrects drift.
    ref.listen(deckPositionMsProvider(widget.deckId), (prev, next) {
      if (_scrubbing || durationMs <= 0) {
        return;
      }
      final display = _displayMs(durationMs);
      final engineMs = next.toDouble();
      final advancingNow = _advancingNow();
      final speedNow = ref.read(deckSpeedRatioProvider(widget.deckId));
      if (playheadShouldSnap(
        displayMs: display,
        engineMs: engineMs,
        playing: advancingNow,
      )) {
        _setDisplayMs(
          engineMs,
          durationMs: durationMs,
          speed: speedNow,
          playing: advancingNow,
        );
        return;
      }
      // Signed out-of-track positions and a live override are authoritative
      // and cannot be represented by the clamped main clock.
      final offTrack = engineMs < 0 || engineMs > durationMs;
      if (offTrack || _outsideMs.value != null) {
        _setDisplayMs(
          engineMs,
          durationMs: durationMs,
          speed: speedNow,
          playing: advancingNow,
        );
        return;
      }
      _correctDrift(
        engineMs: engineMs,
        durationMs: durationMs,
        speed: speedNow,
      );
    });
    ref.listen(deckJogTouchingProvider(widget.deckId), (prev, next) {
      if (_scrubbing || durationMs <= 0) {
        return;
      }
      final advancingNow = playheadAdvancing(
        playing: ref.read(deckPlayingProvider(widget.deckId)),
        jogTouching: next,
      );
      final speedNow = ref.read(deckSpeedRatioProvider(widget.deckId));
      if (!advancingNow) {
        _setDisplayMs(
          ref.read(deckPositionMsProvider(widget.deckId)).toDouble(),
          durationMs: durationMs,
          speed: speedNow,
          playing: false,
        );
        return;
      }
      _syncPlayback(playing: true, durationMs: durationMs, speed: speedNow);
    });
    ref.listen(deckSpeedRatioProvider(widget.deckId), (prev, next) {
      if (durationMs <= 0) {
        return;
      }
      _syncPlayback(
        playing: _advancingNow(),
        durationMs: durationMs,
        speed: next,
      );
    });
    // A different track or a lost duration invalidates the gesture and any
    // final seek still waiting in the queue; display reinit is left to the
    // lane's build and the listeners above.
    ref.listen(deckTrackIdProvider(widget.deckId), (prev, next) {
      if (prev != next) {
        _invalidateScrub();
      }
    });
    ref.listen(deckDurationMsProvider(widget.deckId), (prev, next) {
      if ((next ?? 0) <= 0) {
        _invalidateScrub();
      }
    });
    // A load request bumps the generation before the engine sees it, so this
    // drops a queued release seek that would otherwise land on the replacement
    // track (or on this same track reloaded) before its DeckUpdated arrives.
    // An active press also has to stop previewing the old track's position.
    ref.listen(
      deckLoadGenerationProvider.select((m) => m[widget.deckId] ?? 0),
      (prev, next) {
        if (prev == next) {
          return;
        }
        if (_scrubbing || _scratch != null) {
          _cancelScrub();
        } else {
          _invalidateScrub();
        }
      },
    );

    return TrackEndBlink(
      deckId: widget.deckId,
      child: LayoutBuilder(
        builder: (context, constraints) {
          final width = constraints.maxWidth;
          final height = constraints.maxHeight;
          if (width <= 0 || height <= 0) {
            return const SizedBox.expand();
          }
          // Source time on screen: zoom is shared, tempo widens the source span
          // so the musical span holds.
          final visibleSpanMs = visibleSourceMs(speed, zoomMs);
          final pxPerMs = width / visibleSpanMs;
          final dpr = MediaQuery.maybeOf(context)?.devicePixelRatio ?? 1;

          return Listener(
            onPointerSignal: durationMs <= 0
                ? null
                : (event) {
                    if (event is PointerScrollEvent) {
                      ref
                          .read(waveformZoomProvider.notifier)
                          .zoomBy(event.scrollDelta.dy);
                    }
                  },
            onPointerDown: durationMs <= 0
                ? null
                : (e) {
                    if (e.buttons != kPrimaryButton || _scrubbing) {
                      return;
                    }
                    if (ref.read(deckTrackIdProvider(widget.deckId)) == null) {
                      return;
                    }
                    // An external platter hold already owns this deck.
                    if (ref.read(deckJogTouchingProvider(widget.deckId))) {
                      return;
                    }
                    // Pin the displayed position, then stop: a held pointer
                    // must not keep drifting. This is authoritative, so any
                    // pending correction is dropped rather than folded in.
                    _setDisplayMs(
                      _displayMs(durationMs),
                      durationMs: durationMs,
                      speed: speed,
                      playing: false,
                    );
                    _scrubbing = true;
                    _scrubPointer = e.pointer;
                    _scrubAnchorX = e.localPosition.dx;
                    _scrubAnchorMs = _displayMs(durationMs);
                    _lastScrubMs = _scrubAnchorMs;
                    _lastQueuedTicks = 0;
                  },
            onPointerMove: (e) {
              if (!_scrubbing ||
                  e.pointer != _scrubPointer ||
                  durationMs <= 0) {
                return;
              }
              if (shiftKeyPressed()) {
                _cancelScrub();
                return;
              }
              // A platter hold that started after this press owns the deck now:
              // yielding here keeps this lane from lifting it on release.
              if (_scratch == null &&
                  ref.read(deckJogTouchingProvider(widget.deckId))) {
                _cancelScrub();
                return;
              }
              final ms = _scrubTargetMs(
                dx: e.localPosition.dx - _scrubAnchorX,
                width: width,
                spanMs: visibleSpanMs.toDouble(),
              );
              if (!ms.isFinite) {
                return;
              }
              _showMs(ms, durationMs: durationMs);
              _lastScrubMs = ms;
              // Absolute tick count from the press anchor: its difference from
              // what is already queued carries sub-tick moves and reversals.
              final cumulative = ((ms - _scrubAnchorMs) / vinylMsPerTick())
                  .round();
              final delta = cumulative - _lastQueuedTicks;
              if (delta == 0) {
                return;
              }
              _lastQueuedTicks = cumulative;
              _queueTurn(delta);
            },
            onPointerUp: (e) {
              if (!_scrubbing || e.pointer != _scrubPointer) {
                return;
              }
              if (shiftKeyPressed()) {
                _cancelScrub();
                return;
              }
              if (durationMs <= 0) {
                _finishScrub();
                return;
              }
              final raw = _scrubTargetMs(
                dx: e.localPosition.dx - _scrubAnchorX,
                width: width,
                spanMs: visibleSpanMs.toDouble(),
              );
              final ms = raw.isFinite ? raw : _lastScrubMs;
              _finishScrub(positionMs: ms.round());
              _setDisplayMs(
                ms,
                durationMs: durationMs,
                speed: speed,
                playing: advancing,
              );
            },
            onPointerCancel: (e) {
              if (!_scrubbing || e.pointer != _scrubPointer) {
                return;
              }
              _cancelScrub();
            },
            child: ExcludeSemantics(
              child: ClipRect(
                child: GestureDetector(
                  behavior: HitTestBehavior.opaque,
                  onDoubleTap: durationMs <= 0
                      ? null
                      : () => ref.read(waveformZoomProvider.notifier).reset(),
                  child: Stack(
                    fit: StackFit.expand,
                    children: [
                      ColoredBox(color: theme.colors.waveformBackground),
                      if (trackId != null && durationMs > 0)
                        _LaneStrip(
                          deckId: widget.deckId,
                          trackId: trackId,
                          durationMs: durationMs,
                          visibleSpanMs: visibleSpanMs,
                          height: height,
                          pxPerMs: pxPerMs,
                          playhead: _displayTicks,
                          displayMs: () => _displayMs(durationMs),
                          dpr: dpr,
                          onMenuOpened: _cancelScrub,
                        ),
                      Align(
                        child: ColoredBox(
                          color: theme.colors.foreground.withValues(alpha: 0.9),
                          child: const SizedBox(
                            width: 1,
                            height: double.infinity,
                          ),
                        ),
                      ),
                      if (slipOn && slipShadowMs != null && trackId != null)
                        AnimatedBuilder(
                          animation: _displayTicks,
                          builder: (context, _) {
                            final audibleMs = _displayMs(durationMs);
                            final delta = slipShadowMs - audibleMs;
                            if (delta.abs() < 3) {
                              return const SizedBox.shrink();
                            }
                            // The slip shadow is a static 1 px marker and is
                            // deliberately snapped, unlike the scrolling strip,
                            // which moves in fractional pixels so it can scroll
                            // smoothly at sub-pixel rates.
                            return Positioned(
                              left: snapPx(
                                width / 2 + delta * pxPerMs - 0.5,
                                dpr,
                              ),
                              top: 0,
                              bottom: 0,
                              width: 1,
                              child: ColoredBox(
                                color: theme.colors.primary.withValues(
                                  alpha: 0.65,
                                ),
                              ),
                            );
                          },
                        ),
                      if (widget.label case final label?)
                        Align(
                          alignment: Alignment.centerLeft,
                          child: Padding(
                            padding: const EdgeInsets.symmetric(horizontal: 10),
                            child: Text(
                              label.toUpperCase(),
                              style: theme.typography.display.xs.copyWith(
                                fontFamily: MixarFonts.spaceGrotesk,
                                color: deckInk(
                                  deckAccentFor(widget.deckId) ??
                                      DeckAccent.neutral,
                                  theme.colors.brightness,
                                ).withValues(alpha: 0.55),
                                fontWeight: FontWeight.w700,
                                letterSpacing: 0.6,
                                height: 1,
                              ),
                            ),
                          ),
                        ),
                    ],
                  ),
                ),
              ),
            ),
          );
        },
      ),
    );
  }

  /// Ends the current press, releasing the platter and seeking to [positionMs].
  ///
  /// A null [positionMs] is release-only: Shift, cancel, an unload, a deck
  /// change and teardown all use it. Everything the release needs is captured
  /// here, so a queued command never reads `ref` after unmount.
  ///
  /// The release seek is a permanent waveform destination even with slip on:
  /// slip retimes the platter while it is held, it does not move where a
  /// waveform drag lands.
  void _finishScrub({int? positionMs}) {
    final scratch = _scratch;
    _scratch = null;
    _scrubbing = false;
    _scrubPointer = null;
    _lastQueuedTicks = 0;
    final epoch = _seekEpoch;

    if (scratch != null) {
      _queueScrubCommand(scratch.transport, (engine) async {
        // Release before any seek, even for a stale press: a deck left holding
        // its platter would not respond to the next jog.
        await _releaseScratch(engine, scratch);
        if (positionMs == null || _seekEpoch != epoch) {
          return;
        }
        await engine.seek(deckId: scratch.deckId, positionMs: positionMs);
      });
      return;
    }
    if (positionMs == null || !mounted) {
      return;
    }
    // A tap or a sub-tick drag never touched the platter: only the seek is due.
    final deckId = widget.deckId;
    _queueScrubCommand(ref.read(engineTransportProvider.future), (
      engine,
    ) async {
      if (_seekEpoch != epoch) {
        return;
      }
      await engine.seek(deckId: deckId, positionMs: positionMs);
    });
  }

  /// Releases the platter captured at press, retrying once.
  ///
  /// Only a touch this press actually submitted is released: a press dropped
  /// before its touch-on ran never owned the deck-wide touch flag, and lifting
  /// it would break a controller's hold. A second rejection propagates to the
  /// queue node's error handling, which skips the final seek: a disconnected
  /// engine cannot be guaranteed to release, and the queue must still advance.
  Future<void> _releaseScratch(
    EngineTransport engine,
    _WaveformScratch scratch,
  ) async {
    if (!scratch.touchAttempted || scratch.touchReleased) {
      return;
    }
    try {
      await engine.jogTouch(deckId: scratch.deckId, touching: false);
    } on Object catch (_) {
      await engine.jogTouch(deckId: scratch.deckId, touching: false);
    }
    scratch.touchReleased = true;
  }

  /// Drops the press without seeking and invalidates any queued final seek.
  void _invalidateScrub() {
    _seekEpoch++;
    _finishScrub();
  }

  /// Drops the press and puts the playhead back where the engine says it is.
  void _cancelScrub() {
    if (!_scrubbing && _scratch == null) {
      return;
    }
    _invalidateScrub();
    if (!mounted) {
      return;
    }
    final deckId = widget.deckId;
    final durationMs = ref.read(deckDurationMsProvider(deckId)) ?? 0;
    if (durationMs <= 0 || ref.read(deckTrackIdProvider(deckId)) == null) {
      return;
    }
    _setDisplayMs(
      ref.read(deckPositionMsProvider(deckId)).toDouble(),
      durationMs: durationMs,
      speed: ref.read(deckSpeedRatioProvider(deckId)),
      playing: _advancingNow(),
    );
  }

  /// Queues one signed platter turn, capturing the platter on the first tick.
  ///
  /// A press keeps the same deck and transport for every turn and its release,
  /// even if the lane later points at a different deck. Deltas that arrive
  /// while a turn is already in flight merge into the next one: turns add up,
  /// so the sent value is still the exact net movement and a high-rate pointer
  /// cannot backlog the queue with round-trips.
  void _queueTurn(int delta) {
    final scratch = _scratch ?? _startScratch();
    if (scratch == null) {
      return;
    }
    scratch.pendingTurn += delta;
    if (scratch.turnQueued) {
      return;
    }
    scratch.turnQueued = true;
    _queueScrubCommand(scratch.transport, (engine) async {
      // Take the net movement now: a move that arrives during the call queues
      // its own turn behind this one.
      scratch.turnQueued = false;
      final pending = scratch.pendingTurn;
      scratch.pendingTurn = 0;
      // An invalidated press must not keep turning a deck that has since
      // changed track; a failed touch-on cannot own the platter either.
      if (_seekEpoch != scratch.epoch || !scratch.touchSent || pending == 0) {
        return;
      }
      await engine.jogTurn(deckId: scratch.deckId, delta: pending);
    });
  }

  /// Signed source position for a horizontal drag [dx] from the press anchor.
  ///
  /// The preview and the release target share it so both convert pixels with
  /// the same zoom/tempo span.
  double _scrubTargetMs({
    required double dx,
    required double width,
    required double spanMs,
  }) => centerScrubMs(
    anchorPosMs: _scrubAnchorMs,
    deltaX: dx,
    width: width,
    spanMs: spanMs,
  );

  _WaveformScratch? _startScratch() {
    if (!mounted) {
      return null;
    }
    final scratch = _WaveformScratch(
      deckId: widget.deckId,
      epoch: _seekEpoch,
      transport: ref.read(engineTransportProvider.future),
    );
    _scratch = scratch;
    _queueScrubCommand(scratch.transport, (engine) async {
      // Never start owning the platter for a press that was already dropped.
      if (_seekEpoch != scratch.epoch) {
        return;
      }
      // From here on the engine may hold the platter, so the release owes it a
      // touch-off even if this call rejects.
      scratch.touchAttempted = true;
      await engine.jogTouch(deckId: scratch.deckId, touching: true);
      scratch.touchSent = true;
    });
    return scratch;
  }

  /// Appends one command to this lane's serial transport tail.
  ///
  /// The node catches its own rejection: it reports the error like the old
  /// seek path and still resolves, so the release that follows always runs.
  void _queueScrubCommand(
    Future<EngineTransport?> transport,
    Future<void> Function(EngineTransport) send,
  ) {
    _scrubQueue = _scrubQueue
        .then((_) async {
          try {
            final engine = await transport;
            if (engine == null) {
              return;
            }
            await send(engine);
          } on Object catch (e) {
            _reportScrubError(e);
          }
        })
        // Reporting is best effort: even a throwing toast must not break the
        // tail, or a later release would never run.
        .catchError((Object _) {});
  }

  void _reportScrubError(Object error) {
    if (_disposed || !mounted) {
      return;
    }
    showMixarToast(
      context: context,
      variant: MixarToastVariant.destructive,
      title: Text('$error'),
    );
  }
}

/// Platter state captured on the first nonzero tick of one waveform drag.
class _WaveformScratch {
  new({required this.deckId, required this.epoch, required this.transport});

  final int deckId;

  /// Seek epoch at capture: a change drops this press's not-yet-sent commands.
  final int epoch;
  final Future<EngineTransport?> transport;

  /// True once touch-on has been submitted, so the release owes a touch-off
  /// even if that call rejected. A press invalidated before submission never
  /// attempted a touch and must not lift a hold it does not own.
  bool touchAttempted = false;
  bool touchSent = false;
  bool touchReleased = false;

  /// Tick deltas for this press that have not reached the engine yet, and
  /// whether a turn is already queued to carry them.
  int pendingTurn = 0;
  bool turnQueued = false;
}

/// The windowed waveform picture, positioned so the playhead sits at center.
///
/// Splitting it out keeps the ~30Hz playhead animation away from the lane's
/// build: [WaveformScrollViewport] moves the strip during paint, so a tick
/// neither rebuilds this subtree nor relays it out, and the strip and its
/// overlays are only rebuilt when the window or the strip changes.
class _LaneStrip extends ConsumerWidget {
  const new({
    required this.deckId,
    required this.trackId,
    required this.durationMs,
    required this.visibleSpanMs,
    required this.height,
    required this.pxPerMs,
    required this.playhead,
    required this.displayMs,
    required this.dpr,
    this.onMenuOpened,
  });

  final int deckId;
  final String trackId;
  final int durationMs;
  final int visibleSpanMs;
  final double height;
  final double pxPerMs;
  final Listenable playhead;
  final double Function() displayMs;
  final double dpr;
  final VoidCallback? onMenuOpened;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    // The strip owns its window (origin/span) and rebuilds itself when the
    // playhead slides or the zoom changes, keeping the painted window until the
    // new peaks land. Nothing coarse is ever drawn in between.
    final strip = ref.watch(
      waveformStripProvider((trackId, durationMs, deckId)),
    );
    if (strip == null) {
      return const SizedBox.shrink();
    }
    final stripOrigin = strip.originMs;
    final stripSpan = strip.spanMs;
    final stripWidth = strip.widthPx;
    // Overlays are recorded at the width they are painted at. Recording them at
    // the (fixed) peak-texture width and scaling that picture would resample
    // 1 px loop and cue edges, and an anti-alias-free hairline drawn at a
    // fractional scale drops out at some zoom levels.
    final displayWidth = (stripSpan * pxPerMs).round();
    final savedLoops = ref.watch(deckSavedLoopsProvider(deckId));
    final hotCues = ref.watch(deckHotCuesProvider(deckId));
    final cuePointMs = ref.watch(deckCuePointMsProvider(deckId));
    final activeLoop = ref.watch(deckActiveLoopProvider(deckId));
    final active = activeLoop != null && activeLoop.active ? activeLoop : null;
    final visibleSavedLoops = [
      for (final loop in savedLoops)
        if (active == null ||
            loop.inMs != active.inMs ||
            loop.outMs != active.outMs)
          loop,
    ];
    final accent = context.theme.colors.primary;
    final sections = <WaveformSection>[
      for (final loop in visibleSavedLoops)
        WaveformSection(
          type: WaveformSectionType.loop,
          startMs: loop.inMs,
          endMs: loop.outMs,
          color: accent,
        ),
      if (active != null)
        WaveformSection(
          type: WaveformSectionType.loop,
          startMs: active.inMs,
          endMs: active.outMs,
          color: accent,
          active: true,
        ),
      if (cuePointMs case final cuePoint?)
        WaveformSection(
          type: WaveformSectionType.mainCue,
          startMs: cuePoint,
          color: context.theme.colors.cueOn,
        ),
      for (final cue in hotCues)
        WaveformSection(
          type: WaveformSectionType.cue,
          startMs: cue.positionMs,
          color: hotCueAccent(cue.slot).border.withValues(alpha: 1),
          label: '${cue.slot + 1}',
        ),
    ];
    final pendingLoopIn = ref.watch(
      stripPendingLoopInPictureProvider((
        deckId,
        stripOrigin,
        stripSpan,
        displayWidth,
      )),
    );
    // Wider than the audio window on both sides so the grid still covers the
    // viewport when the strip is clamped at the track's start or end (and a
    // little way into the silence before/after it). Recording it as a picture
    // keeps the per-tick playback path to a translate.
    final gridOrigin = stripOrigin - 2 * visibleSpanMs;
    final gridSpan = stripSpan + 4 * visibleSpanMs;
    final gridWidth = (gridSpan * pxPerMs).round();
    final beatGrid = ref.watch(
      stripBeatGridPictureProvider((trackId, gridOrigin, gridSpan, gridWidth)),
    );
    final beatGridLeft = (gridOrigin - stripOrigin) * pxPerMs;

    return WaveformScrollViewport(
      playhead: playhead,
      displayMs: displayMs,
      originMs: stripOrigin,
      pxPerMs: pxPerMs,
      stripSize: Size(displayWidth.toDouble(), height),
      child: LoopBandDeleteRegion(
        deckId: deckId,
        trackId: trackId,
        loops: visibleSavedLoops,
        originMs: stripOrigin,
        spanMs: stripSpan,
        width: displayWidth.toDouble(),
        height: height,
        onMenuOpened: onMenuOpened,
        child: RepaintBoundary(
          child: Stack(
            fit: StackFit.expand,
            // The grid picture starts before the strip's audio window and may
            // reach past its far edge; the lane's ClipRect keeps it inside the
            // lane.
            clipBehavior: Clip.none,
            children: [
              FittedBox(
                fit: BoxFit.fill,
                child: SizedBox(
                  width: stripWidth.toDouble(),
                  height: height,
                  child: _StripWaveform(
                    deckId: deckId,
                    strip: strip,
                    height: height,
                  ),
                ),
              ),
              if (beatGrid != null)
                Positioned(
                  left: beatGridLeft,
                  top: 0,
                  bottom: 0,
                  width: gridWidth.toDouble(),
                  child: RepaintBoundary(
                    child: CustomPaint(
                      painter: _BeatGridPicturePainter(picture: beatGrid),
                    ),
                  ),
                ),
              CustomPaint(
                painter: _StripOverlayPainter(pendingLoopIn: pendingLoopIn),
              ),
              CustomPaint(
                painter: WaveformSectionPainter(
                  sections: sections,
                  originMs: stripOrigin,
                  spanMs: stripSpan,
                  compact: false,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _StripWaveform extends ConsumerWidget {
  const new({required this.deckId, required this.strip, required this.height});

  final int deckId;
  final WaveformStrip strip;
  final double height;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final channel = ref.watch(deckMixerChannelProvider(deckId));
    final gains = waveformGainsFor(
      channel,
      shaped: ref.watch(waveformGainEqShapingProvider),
    );
    return SizedBox(
      width: strip.widthPx.toDouble(),
      height: height,
      child: CustomPaint(
        painter: _WaveformPainter(
          strip: strip,
          background: context.theme.colors.waveformBackground,
          mode: ref.watch(waveformDisplayModeProvider),
          gain: gains.gain,
          bandGain: gains.bandGain,
        ),
        size: Size(strip.widthPx.toDouble(), height),
      ),
    );
  }
}

/// Draws pictures recorded at [kWaveformStripHeight] onto a strip canvas.
///
/// One place owns how recorded overlays map to the lane, so the grid and the
/// loop/cue layer cannot drift apart if the vertical mapping changes.
void _paintStripPictures(
  Canvas canvas,
  Size size,
  Iterable<Picture?> pictures,
) {
  if (size.width <= 0 || size.height <= 0) {
    return;
  }
  canvas.save();
  canvas.scale(1, size.height / kWaveformStripHeight);
  for (final picture in pictures) {
    if (picture != null) {
      canvas.drawPicture(picture);
    }
  }
  canvas.restore();
}

/// Pending Loop In marker, replayed at the width it was recorded at.
class _StripOverlayPainter extends CustomPainter {
  new({required this.pendingLoopIn});

  final Picture? pendingLoopIn;

  @override
  void paint(Canvas canvas, Size size) {
    _paintStripPictures(canvas, size, [pendingLoopIn]);
  }

  @override
  bool shouldRepaint(_StripOverlayPainter oldDelegate) =>
      !identical(pendingLoopIn, oldDelegate.pendingLoopIn);
}

class _WaveformPainter extends CustomPainter {
  new({
    required this.strip,
    required this.background,
    required this.mode,
    required this.gain,
    required this.bandGain,
  });

  final WaveformStrip strip;
  final Color background;
  final WaveformDisplayMode mode;
  final double gain;
  final WaveformBandGain bandGain;

  @override
  void paint(Canvas canvas, Size size) {
    if (strip.heightPx <= 0) {
      return;
    }
    final sy = size.height / strip.heightPx;
    final laneHeight = strip.heightPx.toDouble();
    canvas.save();
    canvas.scale(1, sy);
    // Painted live, not replayed from a recorded picture: gains and the
    // display mode are uniforms, so a deck's EQ or a settings flip costs one
    // quad instead of a re-recorded strip.
    WaveformShaderPainter(
      texture: strip.texture,
      background: background,
      mode: mode,
      gain: gain,
      bandGain: bandGain,
    ).paint(canvas, Size(strip.widthPx.toDouble(), laneHeight));
    canvas.restore();
  }

  @override
  bool shouldRepaint(_WaveformPainter oldDelegate) =>
      !identical(strip, oldDelegate.strip) ||
      background != oldDelegate.background ||
      mode != oldDelegate.mode ||
      gain != oldDelegate.gain ||
      bandGain != oldDelegate.bandGain;
}

/// Replays the recorded tempo bars at the window they were recorded for.
///
/// The grid picture is wider than the strip (the strip window is clamped at
/// the track edges), anchored in source time, and translated with the strip,
/// so playback only moves this layer instead of rebuilding the marks.
class _BeatGridPicturePainter extends CustomPainter {
  new({required this.picture});

  final Picture picture;

  @override
  void paint(Canvas canvas, Size size) {
    _paintStripPictures(canvas, size, [picture]);
  }

  @override
  bool shouldRepaint(_BeatGridPicturePainter oldDelegate) =>
      !identical(picture, oldDelegate.picture);
}
