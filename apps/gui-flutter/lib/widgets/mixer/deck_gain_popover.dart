import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/widgets/mixer/rotary_knob.dart';
import 'package:gui_flutter/widgets/shell/app_tooltip.dart';
import 'package:gui_flutter/widgets/shell/mixar_popover.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// LUFS / ReplayGain / auto / trim popover (Tauri `DeckInfoPopover`).
class DeckGainPopover extends ConsumerWidget {
  const new({required this.deckId, required this.hasTrack, super.key});

  final int deckId;
  final bool hasTrack;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = context.theme;
    final loudness = ref.watch(deckLoudnessLufsProvider(deckId));
    final autoGain = ref.watch(deckAutoGainDbProvider(deckId));
    final trimDb = normToStripDb(
      ref.watch(deckMixerChannelProvider(deckId)).gainTrim,
    );
    final total = autoGain + trimDb;

    return MixarPopover(
      enabled: hasTrack,
      overlayBuilder: (context) {
        return Padding(
          padding: const EdgeInsets.all(12),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            mainAxisSize: MainAxisSize.min,
            children: [
              Text(
                'Gain',
                style: theme.typography.body.sm.copyWith(
                  fontWeight: FontWeight.w600,
                ),
              ),
              const SizedBox(height: 8),
              _GainRow(label: 'Loudness', value: _formatLufs(loudness)),
              _GainRow(
                label: 'ReplayGain',
                value: loudness == null || !loudness.isFinite
                    ? '—'
                    : _formatGainDb(-18 - loudness),
              ),
              _GainRow(label: 'Auto gain', value: _formatGainDb(autoGain)),
              _GainRow(label: 'Gain trim', value: _formatGainDb(trimDb)),
              _GainRow(label: 'Total gain', value: _formatGainDb(total)),
            ],
          ),
        );
      },
      childBuilder: (context, controller) {
        final theme = context.theme;
        const tip = 'Deck gain details';
        return AppTooltip(
          tip: tip,
          child: GestureDetector(
            onTap: hasTrack ? controller.toggle : null,
            child: Semantics(
              button: true,
              enabled: hasTrack,
              label: tip,
              child: Padding(
                padding: const EdgeInsets.all(4),
                child: Icon(
                  LucideIcons.info,
                  size: 14,
                  color: hasTrack
                      ? theme.colors.mutedForeground
                      : theme.colors.border,
                ),
              ),
            ),
          ),
        );
      },
    );
  }
}

class _GainRow extends StatelessWidget {
  const new({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return Padding(
      padding: const .symmetric(vertical: 2),
      child: Row(
        children: [
          Expanded(
            child: Text(
              label,
              style: theme.typography.body.xs.copyWith(
                color: theme.colors.mutedForeground,
              ),
            ),
          ),
          Text(
            value,
            style: theme.typography.body.xs.copyWith(
              fontWeight: .w600,
              fontFeatures: const [FontFeature.tabularFigures()],
            ),
          ),
        ],
      ),
    );
  }
}

String _formatLufs(double? value) {
  if (value == null || !value.isFinite) {
    return '—';
  }
  return '${value.toStringAsFixed(1)} LUFS';
}

String _formatGainDb(double value) {
  final sign = value > 0 ? '+' : '';
  return '$sign${value.toStringAsFixed(1)} dB';
}
