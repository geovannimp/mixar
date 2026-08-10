import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';

/// Placeholder deck chrome (track info, pads, jog, transport).
class DeckPanel extends StatelessWidget {
  const DeckPanel({required this.label, super.key});

  final String label;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;

    return FCard(
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(label, style: theme.typography.body.sm.copyWith(fontWeight: FontWeight.w700)),
            Text(
              'No track loaded',
              style: theme.typography.body.xs.copyWith(color: theme.colors.mutedForeground),
            ),
            const SizedBox(height: 12),
            Expanded(
              child: Row(
                children: [
                  Expanded(
                    child: _PlaceholderBox(
                      label: 'Pads',
                      child: GridView.count(
                        crossAxisCount: 4,
                        mainAxisSpacing: 4,
                        crossAxisSpacing: 4,
                        physics: const NeverScrollableScrollPhysics(),
                        children: [
                          for (var i = 1; i <= 8; i++)
                            DecoratedBox(
                              decoration: BoxDecoration(
                                border: Border.all(color: theme.colors.border),
                                borderRadius: BorderRadius.circular(6),
                              ),
                              child: Center(child: Text('$i', style: theme.typography.body.xs)),
                            ),
                        ],
                      ),
                    ),
                  ),
                  const SizedBox(width: 8),
                  const Expanded(
                    child: _PlaceholderBox(
                      label: 'Jog',
                      child: Center(child: _JogPlaceholder()),
                    ),
                  ),
                ],
              ),
            ),
            const SizedBox(height: 8),
            Row(
              children: [
                Expanded(
                  child: FButton(
                    variant: .secondary,
                    onPress: () {},
                    child: const Text('Cue'),
                  ),
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: FButton(
                    onPress: () {},
                    child: const Text('Play'),
                  ),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

class _JogPlaceholder extends StatelessWidget {
  const _JogPlaceholder();

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return AspectRatio(
      aspectRatio: 1,
      child: DecoratedBox(
        decoration: BoxDecoration(
          shape: BoxShape.circle,
          border: Border.all(color: theme.colors.border, width: 3),
        ),
        child: Center(
          child: Text(
            'JOG',
            style: theme.typography.body.xs.copyWith(color: theme.colors.mutedForeground),
          ),
        ),
      ),
    );
  }
}

class _PlaceholderBox extends StatelessWidget {
  const _PlaceholderBox({required this.label, required this.child});

  final String label;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return DecoratedBox(
      decoration: BoxDecoration(
        border: Border.all(color: theme.colors.border),
        borderRadius: BorderRadius.circular(8),
      ),
      child: Padding(
        padding: const EdgeInsets.all(8),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(
              label,
              style: theme.typography.body.xs.copyWith(color: theme.colors.mutedForeground),
            ),
            const SizedBox(height: 6),
            Expanded(child: child),
          ],
        ),
      ),
    );
  }
}
