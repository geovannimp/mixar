import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';

/// Center mixer placeholder: EQ columns, faders, crossfader.
class MixerStrip extends StatelessWidget {
  const MixerStrip({super.key});

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;

    return SizedBox(
      width: 200,
      child: FCard(
        child: Padding(
          padding: const EdgeInsets.all(12),
          child: Column(
            children: [
              Text(
                'Mixer',
                style: theme.typography.body.sm.copyWith(fontWeight: FontWeight.w700),
              ),
              const SizedBox(height: 8),
              Expanded(
                child: Row(
                  children: [
                    Expanded(child: _ChannelColumn(label: 'A')),
                    const SizedBox(width: 8),
                    Expanded(child: _ChannelColumn(label: 'B')),
                  ],
                ),
              ),
              const SizedBox(height: 8),
              Text(
                'Crossfader',
                style: theme.typography.body.xs.copyWith(color: theme.colors.mutedForeground),
              ),
              const SizedBox(height: 4),
              const _CrossfaderPlaceholder(),
            ],
          ),
        ),
      ),
    );
  }
}

class _ChannelColumn extends StatelessWidget {
  const _ChannelColumn({required this.label});

  final String label;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return Column(
      children: [
        Text(label, style: theme.typography.body.xs),
        const SizedBox(height: 6),
        for (final name in ['Gain', 'Hi', 'Mid', 'Low']) ...[
          Text(
            name,
            style: theme.typography.body.xs.copyWith(color: theme.colors.mutedForeground),
          ),
          const SizedBox(height: 2),
          DecoratedBox(
            decoration: BoxDecoration(
              shape: BoxShape.circle,
              border: Border.all(color: theme.colors.border),
            ),
            child: const SizedBox(width: 28, height: 28),
          ),
          const SizedBox(height: 6),
        ],
        Expanded(
          child: DecoratedBox(
            decoration: BoxDecoration(
              border: Border.all(color: theme.colors.border),
              borderRadius: BorderRadius.circular(4),
            ),
            child: const SizedBox(width: 28),
          ),
        ),
      ],
    );
  }
}

class _CrossfaderPlaceholder extends StatelessWidget {
  const _CrossfaderPlaceholder();

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return DecoratedBox(
      decoration: BoxDecoration(
        border: Border.all(color: theme.colors.border),
        borderRadius: BorderRadius.circular(4),
      ),
      child: const SizedBox(height: 28, width: double.infinity),
    );
  }
}
