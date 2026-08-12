import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/deck_loop_panel.dart';
import 'package:gui_flutter/mixer/deck_pads_panel.dart';
import 'package:gui_flutter/mixer/performance_modes.dart';

/// Left rail + exclusive Pads / Loop (extensible) content.
class DeckPerformancePanel extends StatefulWidget {
  const DeckPerformancePanel({
    this.hasTrack = false,
    this.disabled = false,
    super.key,
  });

  final bool hasTrack;
  final bool disabled;

  @override
  State<DeckPerformancePanel> createState() => _DeckPerformancePanelState();
}

class _DeckPerformancePanelState extends State<DeckPerformancePanel> {
  DeckPerformanceMode _mode = DeckPerformanceMode.pads;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;

    return DecoratedBox(
      decoration: BoxDecoration(
        border: Border.all(color: theme.colors.border),
        borderRadius: theme.style.borderRadius.md,
        color: theme.colors.background.withValues(alpha: 0.8),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _PerformanceModeRail(
            mode: _mode,
            disabled: widget.disabled,
            onSelect: (next) => setState(() => _mode = next),
          ),
          Expanded(
            child: IndexedStack(
              index: kDeckPerformanceModes.indexOf(_mode),
              sizing: .expand,
              children: [
                DeckPadsPanel(
                  hasTrack: widget.hasTrack,
                  disabled: widget.disabled,
                  bordered: false,
                ),
                DeckLoopPanel(
                  hasTrack: widget.hasTrack,
                  disabled: widget.disabled,
                  bordered: false,
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _PerformanceModeRail extends StatelessWidget {
  const _PerformanceModeRail({
    required this.mode,
    required this.onSelect,
    this.disabled = false,
  });

  final DeckPerformanceMode mode;
  final ValueChanged<DeckPerformanceMode> onSelect;
  final bool disabled;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;

    return DecoratedBox(
      decoration: BoxDecoration(
        border: Border(right: BorderSide(color: theme.colors.border)),
      ),
      child: SizedBox(
        width: 40,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            for (final entry in kDeckPerformanceModes)
              Expanded(
                child: _RailItem(
                  label: deckPerformanceModeLabel(entry),
                  active: mode == entry,
                  disabled: disabled,
                  onPress: () => onSelect(entry),
                ),
              ),
          ],
        ),
      ),
    );
  }
}

class _RailItem extends StatelessWidget {
  const _RailItem({
    required this.label,
    required this.active,
    required this.onPress,
    this.disabled = false,
  });

  final String label;
  final bool active;
  final VoidCallback onPress;
  final bool disabled;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final fg = disabled
        ? theme.colors.mutedForeground.withValues(alpha: 0.45)
        : active
        ? theme.colors.foreground
        : theme.colors.mutedForeground;

    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: disabled ? null : onPress,
      child: ColoredBox(
        color: active
            ? theme.colors.secondary.withValues(alpha: 0.55)
            : const Color(0x00000000),
        child: Center(
          child: RotatedBox(
            quarterTurns: 3,
            child: Text(
              label.toUpperCase(),
              maxLines: 1,
              style: theme.typography.body.xs.copyWith(
                fontWeight: FontWeight.w700,
                letterSpacing: 1.2,
                color: fg,
                fontSize: 10,
              ),
            ),
          ),
        ),
      ),
    );
  }
}
