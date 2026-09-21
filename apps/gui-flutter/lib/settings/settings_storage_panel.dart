import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/settings/settings_field.dart';
import 'package:gui_flutter/settings/settings_widgets.dart';
import 'package:gui_flutter/shell/app_button.dart';
import 'package:gui_flutter/shell/mixar_dialog.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:gui_flutter/shell/mixar_toast.dart';
import 'package:gui_flutter/src/rust/api/library.dart';

/// Category colors for the storage overview bar (readable on light + dark).
const _kStemCacheColor = Color(0xFF0EA5E9);
const _kStemModelColor = Color(0xFFF59E0B);
const _kWaveformColor = Color(0xFF22C55E);
const _kMetadataColor = Color(0xFFF43F5E);

class SettingsStoragePanel extends ConsumerStatefulWidget {
  const new({super.key});

  @override
  ConsumerState<SettingsStoragePanel> createState() =>
      _SettingsStoragePanelState();
}

class _SettingsStoragePanelState extends ConsumerState<SettingsStoragePanel> {
  StorageUsage? _usage;
  var _loading = true;
  var _busy = false;
  String? _error;

  @override
  void initState() {
    super.initState();
    _refresh();
  }

  Future<void> _refresh() async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final transport = await ref.read(libraryTransportProvider.future);
      final usage = await transport.storageUsage();
      if (!mounted) {
        return;
      }
      setState(() {
        _usage = usage;
        _loading = false;
      });
    } catch (e) {
      if (!mounted) {
        return;
      }
      setState(() {
        _error = '$e';
        _loading = false;
      });
    }
  }

  Future<void> _confirmClear({
    required String title,
    required String body,
    required Future<void> Function(LibraryTransport) clear,
    required String okTitle,
  }) async {
    final confirmed = await showMixarConfirm<bool>(
      context: context,
      title: title,
      body: body,
      actions: const [
        MixarDialogAction(
          label: 'Cancel',
          value: false,
          variant: MixarButtonVariant.outline,
        ),
        MixarDialogAction(
          label: 'Clear',
          value: true,
          variant: MixarButtonVariant.destructive,
        ),
      ],
    );
    if (confirmed != true || !mounted) {
      return;
    }
    await _runClear(clear, okTitle);
  }

  Future<void> _runClear(
    Future<void> Function(LibraryTransport) clear,
    String okTitle,
  ) async {
    setState(() => _busy = true);
    try {
      final transport = await ref.read(libraryTransportProvider.future);
      await clear(transport);
      if (!mounted) {
        return;
      }
      showMixarToast(context: context, title: Text(okTitle));
      await _refresh();
    } catch (e) {
      if (!mounted) {
        return;
      }
      showMixarToast(
        context: context,
        title: const Text('Clear failed'),
        description: Text('$e'),
        variant: MixarToastVariant.destructive,
      );
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final usage = _usage;
    final stems = usage?.stemsBytes ?? BigInt.zero;
    final models = usage?.modelsBytes ?? BigInt.zero;
    final waveforms = usage?.waveformBytes ?? BigInt.zero;
    final metadata = usage?.metadataBytes ?? BigInt.zero;
    final total = stems + models + waveforms + metadata;
    final ready = !_loading && usage != null;
    final clearEnabled = ready && !_busy;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      spacing: 16,
      children: [
        const SettingsSectionHeader(
          title: 'Storage',
          description: 'Disk use for Mixar caches and library metadata under app support.',
        ),
        if (_error != null)
          Text(
            _error!,
            style: theme.typography.body.sm.copyWith(
              color: theme.colors.destructive,
            ),
          ),
        SettingsPanel(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            spacing: 16,
            children: [
              Row(
                children: [
                  Expanded(
                    child: Text(
                      'Mixar storage',
                      style: theme.typography.body.sm.copyWith(
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ),
                  Text(
                    ready ? '${formatStorageBytes(total)} used' : '…',
                    style: theme.typography.body.sm.copyWith(
                      color: theme.colors.mutedForeground,
                    ),
                  ),
                ],
              ),
              _StorageUsageBar(
                segments: [
                  (stems, _kStemCacheColor),
                  (models, _kStemModelColor),
                  (waveforms, _kWaveformColor),
                  (metadata, _kMetadataColor),
                ],
                loading: !ready,
              ),
              Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                spacing: 8,
                children: [
                  _StorageLegendRow(
                    color: _kStemCacheColor,
                    label: 'Stem cache',
                    sizeLabel: ready ? formatStorageBytes(stems) : '…',
                    onClear: !clearEnabled
                        ? null
                        : () => _confirmClear(
                            title: 'Clear stem cache?',
                            body: 'Deletes generated stem files for all tracks. Original library audio is not touched.',
                            clear: (t) => t.clearStemCache(),
                            okTitle: 'Stem cache cleared',
                          ),
                  ),
                  _StorageLegendRow(
                    color: _kStemModelColor,
                    label: 'Stem model',
                    sizeLabel: ready ? formatStorageBytes(models) : '…',
                    onClear: !clearEnabled
                        ? null
                        : () => _confirmClear(
                            title: 'Clear stem model cache?',
                            body: 'Deletes downloaded stem separation models. They will re-download when needed.',
                            clear: (t) => t.clearModelCache(),
                            okTitle: 'Stem model cleared',
                          ),
                  ),
                  _StorageLegendRow(
                    color: _kWaveformColor,
                    label: 'Waveform',
                    sizeLabel: ready ? formatStorageBytes(waveforms) : '…',
                    onClear: !clearEnabled
                        ? null
                        : () => _confirmClear(
                            title: 'Clear waveform cache?',
                            body: 'Deletes cached waveform overviews. They regenerate when you open a track.',
                            clear: (t) => t.clearWaveformCache(),
                            okTitle: 'Waveform cache cleared',
                          ),
                  ),
                  _StorageLegendRow(
                    color: _kMetadataColor,
                    label: 'Track metadata',
                    sizeLabel: ready ? formatStorageBytes(metadata) : '…',
                    onClear: null,
                  ),
                ],
              ),
            ],
          ),
        ),
      ],
    );
  }
}

class _StorageUsageBar extends StatelessWidget {
  const new({required this.segments, required this.loading});

  final List<(BigInt bytes, Color color)> segments;
  final bool loading;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final total = segments.fold<BigInt>(BigInt.zero, (sum, s) => sum + s.$1);

    return ClipRRect(
      borderRadius: const BorderRadius.all(Radius.circular(6)),
      child: DecoratedBox(
        decoration: BoxDecoration(
          color: theme.colors.muted,
          border: Border.all(color: theme.colors.border),
          borderRadius: const BorderRadius.all(Radius.circular(6)),
        ),
        child: SizedBox(
          height: 22,
          child: loading || total == BigInt.zero
              ? const SizedBox.expand()
              : Row(
                  children: [
                    for (final (bytes, color) in segments)
                      if (bytes > BigInt.zero)
                        Expanded(
                          flex: _flex(storageShare(bytes, total)),
                          child: ColoredBox(
                            color: color,
                            child: const SizedBox.expand(),
                          ),
                        ),
                  ],
                ),
        ),
      ),
    );
  }

  static int _flex(double fraction) => (fraction * 1000).round().clamp(1, 1000);
}

class _StorageLegendRow extends StatelessWidget {
  const new({
    required this.color,
    required this.label,
    required this.sizeLabel,
    required this.onClear,
  });

  final Color color;
  final String label;
  final String sizeLabel;
  final VoidCallback? onClear;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return Row(
      spacing: 12,
      children: [
        DecoratedBox(
          decoration: BoxDecoration(
            color: color,
            borderRadius: const BorderRadius.all(Radius.circular(3)),
          ),
          child: const SizedBox(width: 10, height: 10),
        ),
        Expanded(
          child: Text(
            label,
            style: theme.typography.body.sm.copyWith(
              fontWeight: FontWeight.w600,
            ),
          ),
        ),
        Text(
          sizeLabel,
          style: theme.typography.body.sm.copyWith(
            color: theme.colors.mutedForeground,
          ),
        ),
        if (onClear != null)
          AppButton(
            variant: MixarButtonVariant.destructive,
            size: MixarButtonSize.sm,
            onPress: onClear,
            child: const Text('Clear'),
          ),
      ],
    );
  }
}

/// Share of [part] within [total], or 0 when total is zero.
double storageShare(BigInt part, BigInt total) {
  if (total == BigInt.zero) {
    return 0;
  }
  return part.toDouble() / total.toDouble();
}

/// Human-readable byte size for Storage settings.
String formatStorageBytes(BigInt bytes) {
  final n = bytes.toUnsigned(64).toInt();
  if (n < 1024) {
    return '$n B';
  }
  if (n < 1024 * 1024) {
    return '${(n / 1024).toStringAsFixed(1)} KB';
  }
  if (n < 1024 * 1024 * 1024) {
    return '${(n / (1024 * 1024)).toStringAsFixed(1)} MB';
  }
  return '${(n / (1024 * 1024 * 1024)).toStringAsFixed(2)} GB';
}
