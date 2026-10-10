import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
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

  Future<void> _confirmSyncStems() async {
    final l10n = AppLocalizations.of(context)!;
    final confirmed = await showMixarConfirm<bool>(
      context: context,
      title: l10n.settingsStorageSyncStemTitle,
      body: l10n.settingsStorageSyncStemBody,
      actions: [
        MixarDialogAction(
          label: l10n.settingsCancel,
          value: false,
          variant: MixarButtonVariant.outline,
        ),
        MixarDialogAction(
          label: l10n.commonSync,
          value: true,
          variant: MixarButtonVariant.primary,
        ),
      ],
    );
    if (confirmed != true || !mounted) {
      return;
    }
    setState(() => _busy = true);
    try {
      final transport = await ref.read(libraryTransportProvider.future);
      final removed = await transport.syncStemCache();
      if (!mounted) {
        return;
      }
      final toastL10n = AppLocalizations.of(context)!;
      showMixarToast(
        context: context,
        title: Text(
          removed == 0
              ? toastL10n.settingsStorageStemAlreadyInSync
              : toastL10n.settingsStorageRemovedOrphans(removed),
        ),
      );
      await _refresh();
    } catch (e) {
      if (!mounted) {
        return;
      }
      showMixarToast(
        context: context,
        title: Text(AppLocalizations.of(context)!.settingsStorageSyncFailed),
        description: Text('$e'),
        variant: MixarToastVariant.destructive,
      );
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  Future<void> _confirmClear({
    required String title,
    required String body,
    required Future<void> Function(LibraryTransport) clear,
    required String okTitle,
  }) async {
    final l10n = AppLocalizations.of(context)!;
    final confirmed = await showMixarConfirm<bool>(
      context: context,
      title: title,
      body: body,
      actions: [
        MixarDialogAction(
          label: l10n.settingsCancel,
          value: false,
          variant: MixarButtonVariant.outline,
        ),
        MixarDialogAction(
          label: l10n.commonClear,
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
        title: Text(AppLocalizations.of(context)!.settingsStorageClearFailed),
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
    final l10n = AppLocalizations.of(context)!;
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
        SettingsSectionHeader(
          title: l10n.settingsSectionStorage,
          description: l10n.settingsStorageDescription,
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
                      l10n.settingsStorageMixarStorage,
                      style: theme.typography.body.sm.copyWith(
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ),
                  Text(
                    ready
                        ? l10n.settingsStorageUsed(formatStorageBytes(total))
                        : '…',
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
                    label: l10n.settingsStorageStemCache,
                    sizeLabel: ready ? formatStorageBytes(stems) : '…',
                    onSync: !clearEnabled ? null : () => _confirmSyncStems(),
                    onClear: !clearEnabled
                        ? null
                        : () => _confirmClear(
                            title: l10n.settingsStorageClearStemTitle,
                            body: l10n.settingsStorageClearStemBody,
                            clear: (t) => t.clearStemCache(),
                            okTitle: l10n.settingsStorageClearStemOk,
                          ),
                  ),
                  _StorageLegendRow(
                    color: _kStemModelColor,
                    label: l10n.settingsStorageStemModel,
                    sizeLabel: ready ? formatStorageBytes(models) : '…',
                    onSync: null,
                    onClear: !clearEnabled
                        ? null
                        : () => _confirmClear(
                            title: l10n.settingsStorageClearModelTitle,
                            body: l10n.settingsStorageClearModelBody,
                            clear: (t) => t.clearModelCache(),
                            okTitle: l10n.settingsStorageClearModelOk,
                          ),
                  ),
                  _StorageLegendRow(
                    color: _kWaveformColor,
                    label: l10n.settingsStorageWaveform,
                    sizeLabel: ready ? formatStorageBytes(waveforms) : '…',
                    onSync: null,
                    onClear: !clearEnabled
                        ? null
                        : () => _confirmClear(
                            title: l10n.settingsStorageClearWaveformTitle,
                            body: l10n.settingsStorageClearWaveformBody,
                            clear: (t) => t.clearWaveformCache(),
                            okTitle: l10n.settingsStorageClearWaveformOk,
                          ),
                  ),
                  _StorageLegendRow(
                    color: _kMetadataColor,
                    label: l10n.settingsStorageTrackMetadata,
                    sizeLabel: ready ? formatStorageBytes(metadata) : '…',
                    onSync: null,
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
    required this.onSync,
    required this.onClear,
  });

  final Color color;
  final String label;
  final String sizeLabel;
  final VoidCallback? onSync;
  final VoidCallback? onClear;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final l10n = AppLocalizations.of(context)!;
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
        if (onSync != null)
          AppButton(
            variant: MixarButtonVariant.outline,
            size: MixarButtonSize.sm,
            onPress: onSync,
            child: Text(l10n.commonSync),
          ),
        if (onClear != null)
          AppButton(
            variant: MixarButtonVariant.destructive,
            size: MixarButtonSize.sm,
            onPress: onClear,
            child: Text(l10n.commonClear),
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
