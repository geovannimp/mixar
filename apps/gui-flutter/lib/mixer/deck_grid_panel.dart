import 'package:flutter/gestures.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/pad_modes.dart';
import 'package:gui_flutter/mixer/tempo_format.dart';
import 'package:gui_flutter/mixer/waveform/beat_grid.dart';
import 'package:gui_flutter/shell/app_tooltip.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Beat-grid edit panel for the performance surface.
///
/// ```
/// Grid
/// Tempo
/// −  [128.00]  +
/// Downbeat
/// ⇤  Beat 1  ⇥
/// ```
class DeckGridPanel extends StatelessWidget {
  const DeckGridPanel({
    required this.bpm,
    required this.onSetDownbeat,
    required this.onNudgeBack,
    required this.onNudgeForward,
    required this.onBpmDown,
    required this.onBpmUp,
    required this.onBpmSubmit,
    this.hasTrack = false,
    this.disabled = false,
    this.bordered = true,
    super.key,
  });

  final double? bpm;
  final VoidCallback onSetDownbeat;
  final VoidCallback onNudgeBack;
  final VoidCallback onNudgeForward;
  final VoidCallback onBpmDown;
  final VoidCallback onBpmUp;
  final ValueChanged<double> onBpmSubmit;
  final bool hasTrack;
  final bool disabled;
  final bool bordered;

  bool get _controlsDisabled => disabled || !hasTrack;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final controlsDisabled = _controlsDisabled;
    final chipStyle = theme.typography.body.sm.copyWith(
      fontWeight: FontWeight.w700,
    );

    Widget sideCell({
      required Widget child,
      required VoidCallback? onPress,
      required String tip,
    }) {
      return AspectRatio(
        aspectRatio: 1,
        child: AppTooltip(
          tip: tip,
          child: FButton(
            variant: .outline,
            size: .xs,
            semanticsLabel: tip,
            onPress: controlsDisabled ? null : onPress,
            child: child,
          ),
        ),
      );
    }

    Widget centerCell({
      required Widget child,
      required VoidCallback? onPress,
      int flex = 2,
    }) {
      return Expanded(
        flex: flex,
        child: FButton(
          variant: .outline,
          size: .xs,
          onPress: controlsDisabled ? null : onPress,
          child: child,
        ),
      );
    }

    Widget textCell({
      required String label,
      required VoidCallback? onPress,
      int flex = 2,
    }) => centerCell(
      onPress: onPress,
      flex: flex,
      child: Text(label, style: chipStyle, textAlign: TextAlign.center),
    );

    final controls = ConstrainedBox(
      constraints: const BoxConstraints(maxWidth: 280),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            'Grid',
            textAlign: TextAlign.center,
            style: theme.typography.body.sm.copyWith(
              fontWeight: FontWeight.w700,
            ),
          ),
          const SizedBox(height: 12),
          Text(
            'Tempo',
            textAlign: TextAlign.center,
            style: theme.typography.body.xs,
          ),
          const SizedBox(height: 4),
          IntrinsicHeight(
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                sideCell(
                  tip: 'Decrease BPM',
                  child: const Icon(LucideIcons.minus, size: 16),
                  onPress: onBpmDown,
                ),
                const SizedBox(width: 8),
                Expanded(
                  flex: 2,
                  child: _BpmField(
                    bpm: bpm,
                    enabled: !controlsDisabled,
                    onSubmit: onBpmSubmit,
                  ),
                ),
                const SizedBox(width: 8),
                sideCell(
                  tip: 'Increase BPM',
                  child: const Icon(LucideIcons.plus, size: 16),
                  onPress: onBpmUp,
                ),
              ],
            ),
          ),
          const FDivider(),
          Text(
            'Downbeat',
            textAlign: TextAlign.center,
            style: theme.typography.body.xs,
          ),
          const SizedBox(height: 4),
          IntrinsicHeight(
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                sideCell(
                  tip: 'Nudge grid back',
                  onPress: onNudgeBack,
                  child: const Icon(LucideIcons.arrowLeftFromLine, size: 16),
                ),
                const SizedBox(width: 8),
                textCell(label: 'Now', onPress: onSetDownbeat),
                const SizedBox(width: 8),
                sideCell(
                  tip: 'Nudge grid forward',
                  onPress: onNudgeForward,
                  child: const Icon(LucideIcons.arrowRightFromLine, size: 16),
                ),
              ],
            ),
          ),
        ],
      ),
    );

    final body = Padding(
      padding: const EdgeInsets.all(16),
      child: Center(child: controls),
    );

    if (!bordered) {
      return body;
    }

    return DecoratedBox(
      decoration: BoxDecoration(
        border: Border.all(color: theme.colors.border),
        borderRadius: theme.style.borderRadius.md,
        color: theme.colors.background.withValues(alpha: 0.8),
      ),
      child: body,
    );
  }
}

class _BpmField extends StatefulWidget {
  const _BpmField({
    required this.bpm,
    required this.enabled,
    required this.onSubmit,
  });

  final double? bpm;
  final bool enabled;
  final ValueChanged<double> onSubmit;

  @override
  State<_BpmField> createState() => _BpmFieldState();
}

class _BpmFieldState extends State<_BpmField> {
  late final TextEditingController _controller;
  late final FocusNode _focus;

  @override
  void initState() {
    super.initState();
    _controller = TextEditingController(text: _display(widget.bpm));
    _focus = FocusNode()..addListener(_onFocusChange);
  }

  @override
  void didUpdateWidget(covariant _BpmField oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!_focus.hasFocus && widget.bpm != oldWidget.bpm) {
      _controller.text = _display(widget.bpm);
    }
  }

  @override
  void dispose() {
    _focus
      ..removeListener(_onFocusChange)
      ..dispose();
    _controller.dispose();
    super.dispose();
  }

  void _onFocusChange() {
    if (!_focus.hasFocus) {
      _commit();
    }
  }

  void _commit() {
    final parsed = parseGridBpm(_controller.text);
    if (parsed == null) {
      _controller.text = _display(widget.bpm);
      return;
    }
    _controller.text = formatBpm(parsed);
    if (widget.bpm == null || (widget.bpm! - parsed).abs() > 1e-6) {
      widget.onSubmit(parsed);
    }
  }

  void _onScroll(PointerScrollEvent event) {
    if (!widget.enabled || event.scrollDelta.dy == 0) {
      return;
    }
    final step = shiftKeyPressed() ? kGridBpmCoarseStep : kGridBpmStep;
    final base = parseGridBpm(_controller.text) ?? widget.bpm ?? defaultGridBpm;
    final next = stepGridBpm(base, event.scrollDelta.dy < 0 ? step : -step);
    _controller.text = formatBpm(next);
    widget.onSubmit(next);
  }

  static String _display(double? bpm) => bpm == null ? '' : formatBpm(bpm);

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final bpmTextStyle = theme.typography.body.sm.copyWith(
      fontWeight: FontWeight.w700,
      fontFeatures: const [FontFeature.tabularFigures()],
    );

    return Listener(
      onPointerSignal: (signal) {
        if (signal is PointerScrollEvent) {
          _onScroll(signal);
        }
      },
      child: FTextField(
        enabled: widget.enabled,
        hint: 'BPM',
        textAlign: TextAlign.center,
        keyboardType: const TextInputType.numberWithOptions(decimal: true),
        textInputAction: TextInputAction.done,
        inputFormatters: [FilteringTextInputFormatter.allow(RegExp(r'[0-9.]'))],
        style: .delta(
          contentTextStyle: FVariantsDelta.delta([
            FVariantOperation.all(TextStyleDelta.value(bpmTextStyle)),
          ]),
        ),
        control: .managed(controller: _controller),
        focusNode: _focus,
        onSubmit: (_) => _commit(),
      ),
    );
  }
}
