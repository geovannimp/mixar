import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/deck_pads_panel.dart';
import 'package:gui_flutter/mixer/deck_tempo_panel.dart';
import 'package:gui_flutter/mixer/fader_slider.dart';

/// Placeholder deck chrome (track info, pads, jog, transport) + tempo column.
class DeckPanel extends StatelessWidget {
  const DeckPanel({
    required this.label,
    required this.accent,
    required this.isMaster,
    required this.onMasterChanged,
    super.key,
  });

  final String label;
  final FaderAccent accent;
  final bool isMaster;
  final ValueChanged<bool> onMasterChanged;

  bool get _tempoOnRight => accent == FaderAccent.a;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final accentColor = FaderColors.forAccent(accent).grip;
    final tempo = DeckTempoPanel(
      accent: accent,
      isMaster: isMaster,
      onMasterChanged: onMasterChanged,
    );

    final body = Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text(
          label,
          style: theme.typography.body.sm.copyWith(
            fontWeight: FontWeight.w700,
            color: accentColor,
          ),
        ),
        Text(
          'No track loaded',
          style: theme.typography.body.xs.copyWith(
            color: theme.colors.mutedForeground,
          ),
        ),
        const SizedBox(height: 12),
        Expanded(
          child: Row(
            children: [
              const Expanded(
                child: DeckPadsPanel(hasTrack: false),
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
              child: FButton(onPress: () {}, child: const Text('Play')),
            ),
          ],
        ),
      ],
    );

    return FCard(
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            if (!_tempoOnRight) ...[tempo, const SizedBox(width: 8)],
            Expanded(child: body),
            if (_tempoOnRight) ...[const SizedBox(width: 8), tempo],
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
            style: theme.typography.body.xs.copyWith(
              color: theme.colors.mutedForeground,
            ),
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
              style: theme.typography.body.xs.copyWith(
                color: theme.colors.mutedForeground,
              ),
            ),
            const SizedBox(height: 6),
            Expanded(child: child),
          ],
        ),
      ),
    );
  }
}
