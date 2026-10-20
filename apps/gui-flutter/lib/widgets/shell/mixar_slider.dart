import 'package:flutter/widgets.dart';
import 'package:shadcn_ui/shadcn_ui.dart';

/// Mixar slider over [ShadSlider].
///
/// [ShadSlider] reads `initialValue` only when its state is created, so a
/// value that arrives from outside (an engine echo, e.g. the DDJ-400 LEVEL
/// knob) never moves the thumb and the control looks dead. Own the controller
/// instead and mirror [value] into it whenever the widget rebuilds.
class MixarSlider extends StatefulWidget {
  const new({
    required this.value,
    required this.onChanged,
    super.key,
    this.min = 0,
    this.max = 1,
    this.divisions,
    this.enabled = true,
    this.semanticFormatterCallback,
    this.trackHeight,
    this.thumbRadius,
  });

  final double value;
  final ValueChanged<double> onChanged;
  final double min;
  final double max;
  final int? divisions;
  final bool enabled;
  final String Function(double value)? semanticFormatterCallback;

  /// Track thickness; `null` uses the theme default (8).
  final double? trackHeight;

  /// Thumb radius; `null` uses the theme default (10). Compact rows pass
  /// smaller values so the thumb stops dominating the strip.
  final double? thumbRadius;

  @override
  State<MixarSlider> createState() => _MixarSliderState();
}

class _MixarSliderState extends State<MixarSlider> {
  late final ShadSliderController _controller = ShadSliderController(
    initialValue: _clampedValue,
  );

  /// True between [ShadSlider]'s drag start and end.
  bool _dragging = false;

  double get _clampedValue => widget.value.clamp(widget.min, widget.max);

  @override
  void didUpdateWidget(covariant MixarSlider oldWidget) {
    super.didUpdateWidget(oldWidget);
    final external =
        oldWidget.value != widget.value ||
        oldWidget.min != widget.min ||
        oldWidget.max != widget.max;
    // Mid-drag the model lags the finger, so only an external change may move
    // the thumb. Outside a drag the model is authoritative: mirror it even when
    // its value did not change, which reconciles the drift a command leaves
    // behind when the engine ignores it (soft takeover, failed publish).
    if (!external && _dragging) {
      return;
    }
    final next = _clampedValue;
    if (_controller.value != next) {
      _controller.value = next;
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return ShadSlider(
      controller: _controller,
      min: widget.min,
      max: widget.max,
      divisions: widget.divisions,
      enabled: widget.enabled,
      trackHeight: widget.trackHeight,
      thumbRadius: widget.thumbRadius,
      onChanged: widget.onChanged,
      onChangeStart: (_) => _dragging = true,
      onChangeEnd: (_) => _dragging = false,
      semanticFormatterCallback: widget.semanticFormatterCallback,
    );
  }
}
