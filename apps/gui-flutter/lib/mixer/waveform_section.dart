import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';

/// Dual-lane waveform placeholder (Deck A over Deck B).
class WaveformSection extends StatelessWidget {
  const WaveformSection({super.key});

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;

    return ColoredBox(
      color: theme.colors.background,
      child: Stack(
        children: [
          Column(
            children: [
              Expanded(child: _Lane(label: 'Deck A')),
              FDivider(),
              Expanded(child: _Lane(label: 'Deck B')),
            ],
          ),
          Center(
            child: Text(
              'Load tracks to see waveforms.',
              style: theme.typography.body.sm.copyWith(
                color: theme.colors.mutedForeground,
              ),
            ),
          ),
          Align(
            alignment: Alignment.center,
            child: ColoredBox(
              color: theme.colors.foreground.withValues(alpha: 0.35),
              child: const SizedBox(width: 1, height: double.infinity),
            ),
          ),
        ],
      ),
    );
  }
}

class _Lane extends StatelessWidget {
  const _Lane({required this.label});

  final String label;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return Align(
      alignment: Alignment.centerLeft,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 10),
        child: Text(
          label,
          style: theme.typography.body.xs.copyWith(
            color: theme.colors.mutedForeground,
            fontWeight: FontWeight.w600,
          ),
        ),
      ),
    );
  }
}
