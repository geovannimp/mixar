import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/waveform/spectral_color.dart';
import 'package:gui_flutter/mixer/waveform/waveform_providers.dart';
import 'package:gui_flutter/shell/controller_providers.dart';
import 'package:gui_flutter/src/rust/api/controller.dart';

class SettingsPage extends ConsumerWidget {
  const SettingsPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return Align(
      alignment: Alignment.topLeft,
      child: SingleChildScrollView(
        padding: const EdgeInsets.all(24),
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 520),
          child: const Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              _WaveformCard(),
              SizedBox(height: 16),
              _ControllersCard(),
            ],
          ),
        ),
      ),
    );
  }
}

class _WaveformCard extends ConsumerWidget {
  const _WaveformCard();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = context.theme;
    final mode = ref.watch(waveformDisplayModeProvider);
    return FCard(
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
    );
  }
}

class _ControllersCard extends ConsumerStatefulWidget {
  const _ControllersCard();

  @override
  ConsumerState<_ControllersCard> createState() => _ControllersCardState();
}

class _ControllersCardState extends ConsumerState<_ControllersCard> {
  var _busy = false;

  Future<void> _run(Future<void> Function() action) async {
    setState(() => _busy = true);
    try {
      await action();
      ref.invalidate(controllerMappingsProvider);
      ref.invalidate(controllerDevicesProvider);
    } catch (e) {
      if (mounted) {
        showFToast(context: context, variant: .destructive, title: Text('$e'));
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final mappings = ref.watch(controllerMappingsProvider);
    final devices = ref.watch(controllerDevicesProvider);
    final transport = ref.watch(controllerTransportProvider).value;
    return FCard(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(
              'Controllers',
              style: theme.typography.body.md.copyWith(
                fontWeight: FontWeight.w600,
              ),
            ),
            const SizedBox(height: 4),
            Text(
              'MIDI mappings live in app data. Seed copies shipped maps when missing; Update overwrites from the app bundle.',
              style: theme.typography.body.sm.copyWith(
                color: theme.colors.mutedForeground,
              ),
            ),
            const SizedBox(height: 12),
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: [
                FButton(
                  variant: .outline,
                  size: .sm,
                  mainAxisSize: .min,
                  onPress: transport == null || _busy
                      ? null
                      : () => _run(transport.updateAllMappings),
                  child: const Text('Update all mappings'),
                ),
                FButton(
                  variant: .outline,
                  size: .sm,
                  mainAxisSize: .min,
                  onPress: _busy
                      ? null
                      : () {
                          ref.invalidate(controllerMappingsProvider);
                          ref.invalidate(controllerDevicesProvider);
                        },
                  child: const Text('Refresh'),
                ),
              ],
            ),
            const SizedBox(height: 12),
            Text(
              'Mappings',
              style: theme.typography.body.sm.copyWith(
                fontWeight: FontWeight.w600,
              ),
            ),
            const SizedBox(height: 8),
            mappings.when(
              data: (rows) {
                if (rows.isEmpty) {
                  return Text(
                    'No mappings in app data yet.',
                    style: theme.typography.body.sm.copyWith(
                      color: theme.colors.mutedForeground,
                    ),
                  );
                }
                return Column(
                  children: [
                    for (final mapping in rows)
                      _MappingRow(
                        mapping: mapping,
                        busy: _busy || transport == null,
                        onEnable: () => _run(
                          () => transport!.enableMapping(mappingId: mapping.id),
                        ),
                        onDisable: () => _run(
                          () =>
                              transport!.disableMapping(mappingId: mapping.id),
                        ),
                        onUpdate: () => _run(
                          () => transport!.updateMapping(mappingId: mapping.id),
                        ),
                      ),
                  ],
                );
              },
              loading: () => Text(
                'Loading…',
                style: theme.typography.body.sm.copyWith(
                  color: theme.colors.mutedForeground,
                ),
              ),
              error: (e, _) => Text(
                '$e',
                style: theme.typography.body.sm.copyWith(
                  color: theme.colors.destructive,
                ),
              ),
            ),
            const SizedBox(height: 12),
            Text(
              'MIDI ports',
              style: theme.typography.body.sm.copyWith(
                fontWeight: FontWeight.w600,
              ),
            ),
            const SizedBox(height: 8),
            devices.when(
              data: (rows) {
                if (rows.isEmpty) {
                  return Text(
                    'No MIDI ports detected.',
                    style: theme.typography.body.sm.copyWith(
                      color: theme.colors.mutedForeground,
                    ),
                  );
                }
                return Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    for (final device in rows)
                      Text(
                        '${_directionLabel(device.direction)} ${device.portName}'
                        '${device.matchedMappingId != null ? ' → ${device.matchedMappingId}' : ''}',
                        style: theme.typography.body.sm.copyWith(
                          color: theme.colors.mutedForeground,
                        ),
                      ),
                  ],
                );
              },
              loading: () => Text(
                'Loading…',
                style: theme.typography.body.sm.copyWith(
                  color: theme.colors.mutedForeground,
                ),
              ),
              error: (e, _) => Text(
                '$e',
                style: theme.typography.body.sm.copyWith(
                  color: theme.colors.destructive,
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _MappingRow extends StatelessWidget {
  const _MappingRow({
    required this.mapping,
    required this.busy,
    required this.onEnable,
    required this.onDisable,
    required this.onUpdate,
  });

  final ControllerMappingInfo mapping;
  final bool busy;
  final VoidCallback onEnable;
  final VoidCallback onDisable;
  final VoidCallback onUpdate;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final name = [
      mapping.vendorName,
      mapping.productName,
    ].where((s) => s.isNotEmpty).join(' ');
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 6),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text.rich(
            TextSpan(
              text: name,
              children: [
                if (mapping.attached)
                  TextSpan(
                    text: '  attached',
                    style: theme.typography.body.sm.copyWith(
                      fontWeight: FontWeight.w400,
                      color: theme.colors.primary,
                    ),
                  ),
              ],
            ),
            style: theme.typography.body.sm.copyWith(
              fontWeight: FontWeight.w600,
            ),
          ),
          Text(
            '${mapping.id} · ${mapping.deviceId}',
            style: theme.typography.body.sm.copyWith(
              color: theme.colors.mutedForeground,
            ),
          ),
          const SizedBox(height: 6),
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: [
              if (mapping.attached)
                FButton(
                  variant: .outline,
                  size: .sm,
                  mainAxisSize: .min,
                  onPress: busy ? null : onDisable,
                  child: const Text('Disable'),
                )
              else
                FButton(
                  variant: .outline,
                  size: .sm,
                  mainAxisSize: .min,
                  onPress: busy ? null : onEnable,
                  child: const Text('Enable'),
                ),
              FButton(
                variant: .outline,
                size: .sm,
                mainAxisSize: .min,
                onPress: busy ? null : onUpdate,
                child: const Text('Update'),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

String _directionLabel(ControllerDeviceDirection direction) {
  return switch (direction) {
    ControllerDeviceDirection.input => 'input',
    ControllerDeviceDirection.output => 'output',
  };
}
