import 'dart:math' as math;

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/providers/mixer/track_end_warning.dart';

/// Blinks [child] once per warning period while the deck is inside its
/// end-of-track window. The period is two bars of the deck's tempo — see
/// [deckEndWarningBlinkPeriodProvider].
///
/// The blink is a short composited fade, so a warning deck costs no repaint of
/// the child: the waveform strip and the track card keep their own layers.
class TrackEndBlink extends ConsumerWidget {
  const new({
    required this.deckId,
    required this.child,
    this.minOpacity = 0.55,
    super.key,
  });

  final int deckId;
  final Widget child;

  /// Opacity at the dimmest point of the blink. Kept close to opaque on
  /// purpose — the cue has to catch the eye, not black out the deck.
  ///
  /// Each surface picks its own floor: the waveform lane takes the default
  /// 0.55, while the track card passes a higher value so the title and the
  /// times stay readable through the dip.
  final double minOpacity;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return _BlinkPulse(
      active: ref.watch(deckTrackEndWarningProvider(deckId)),
      period: ref.watch(deckEndWarningBlinkPeriodProvider(deckId)),
      minOpacity: minOpacity,
      // Reduce-motion users get a steady dim instead of a strobing deck.
      animate: !(MediaQuery.maybeDisableAnimationsOf(context) ?? false),
      child: child,
    );
  }
}

class _BlinkPulse extends StatefulWidget {
  const new({
    required this.active,
    required this.period,
    required this.minOpacity,
    required this.animate,
    required this.child,
  });

  final bool active;

  /// Distance between two blinks, from the deck's tempo.
  final Duration period;
  final double minOpacity;
  final bool animate;
  final Widget child;

  @override
  State<_BlinkPulse> createState() => _BlinkPulseState();
}

class _BlinkPulseState extends State<_BlinkPulse>
    with SingleTickerProviderStateMixin {
  /// Length of one fade edge: 150 ms, shrunk to 15 % of a shorter period so a
  /// fast half bar still holds full opacity between blinks.
  static Duration _edge(Duration period) {
    final share = period.inMicroseconds * 15 ~/ 100;
    return Duration(microseconds: math.min(150000, share));
  }

  late final AnimationController _controller = AnimationController(
    vsync: this,
    duration: widget.period,
  );
  late Animation<double> _opacity = _controller.drive(
    _sequence(widget.period, widget.minOpacity),
  );

  /// Full opacity, a short dip and back, then steady until the next period.
  static TweenSequence<double> _sequence(Duration period, double minOpacity) {
    final edgeUs = _edge(period).inMicroseconds.toDouble();
    final restUs = math.max(period.inMicroseconds - 2 * edgeUs, edgeUs);
    return TweenSequence<double>([
      TweenSequenceItem(
        tween: Tween<double>(begin: 1, end: minOpacity),
        weight: edgeUs,
      ),
      TweenSequenceItem(
        tween: Tween<double>(begin: minOpacity, end: 1),
        weight: edgeUs,
      ),
      TweenSequenceItem(tween: ConstantTween<double>(1), weight: restUs),
    ]);
  }

  /// Controller value that sits on the bottom of the dip.
  static double _dipValue(Duration period) {
    final edgeUs = _edge(period).inMicroseconds.toDouble();
    final restUs = math.max(period.inMicroseconds - 2 * edgeUs, edgeUs);
    return edgeUs / (edgeUs + edgeUs + restUs);
  }

  @override
  void initState() {
    super.initState();
    _sync();
  }

  @override
  void didUpdateWidget(_BlinkPulse oldWidget) {
    super.didUpdateWidget(oldWidget);
    final retimed =
        oldWidget.period != widget.period ||
        oldWidget.minOpacity != widget.minOpacity;
    if (retimed) {
      // Stop first: a running controller must not carry the old simulation
      // into a new period. `_sync` below restarts the cycle at full opacity.
      _controller
        ..stop()
        ..duration = widget.period;
      _opacity = _controller.drive(_sequence(widget.period, widget.minOpacity));
    }
    if (retimed ||
        oldWidget.active != widget.active ||
        oldWidget.animate != widget.animate) {
      _sync();
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  void _sync() {
    if (!widget.active) {
      _controller
        ..stop()
        ..value = 0;
      return;
    }
    if (!widget.animate) {
      // Park on the dip: a steady dim instead of a strobe.
      _controller
        ..stop()
        ..value = _dipValue(widget.period);
      return;
    }
    // Restart at full opacity: the dip follows one edge later, ~150 ms in, so
    // entering the window reads as an immediate blink.
    _controller
      ..stop()
      ..value = 0
      ..repeat();
  }

  @override
  Widget build(BuildContext context) {
    // The boundary keeps the deck's pixels in their own layer, so the blink
    // only re-composites them with a new alpha instead of repainting the
    // waveform or the card every frame for the whole warning window.
    return FadeTransition(
      opacity: _opacity,
      child: RepaintBoundary(child: widget.child),
    );
  }
}
