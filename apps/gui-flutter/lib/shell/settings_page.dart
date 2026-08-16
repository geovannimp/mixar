import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/waveform/spectral_color.dart';
import 'package:gui_flutter/mixer/waveform/waveform_providers.dart';

class SettingsPage extends ConsumerWidget {
  const SettingsPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = context.theme;
    final mode = ref.watch(waveformDisplayModeProvider);
    return Align(
      alignment: Alignment.topLeft,
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 520),
          child: FCard(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                mainAxisSize: MainAxisSize.min,
                children: [
                  Text(
                    'Waveform',
                    style: theme.typography.body.md.copyWith(
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  const SizedBox(height: 4),
                  Text(
                    'RGB mixes low/mid/high into one color. Filtered stacks the three bands.',
                    style: theme.typography.body.sm.copyWith(
                      color: theme.colors.mutedForeground,
                    ),
                  ),
                  const SizedBox(height: 12),
                  Row(
                    children: [
                      FButton(
                        variant: mode == WaveformDisplayMode.rgb
                            ? .secondary
                            : .outline,
                        size: .sm,
                        mainAxisSize: .min,
                        onPress: () => ref
                            .read(waveformDisplayModeProvider.notifier)
                            .set(WaveformDisplayMode.rgb),
                        child: const Text('RGB'),
                      ),
                      const SizedBox(width: 8),
                      FButton(
                        variant: mode == WaveformDisplayMode.filtered
                            ? .secondary
                            : .outline,
                        size: .sm,
                        mainAxisSize: .min,
                        onPress: () => ref
                            .read(waveformDisplayModeProvider.notifier)
                            .set(WaveformDisplayMode.filtered),
                        child: const Text('Filtered'),
                      ),
                    ],
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}
