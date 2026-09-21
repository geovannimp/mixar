import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/settings/settings_widgets.dart';
import 'package:gui_flutter/shell/app_button.dart';
import 'package:gui_flutter/shell/mixar_dialog.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:gui_flutter/shell/mixar_toast.dart';
import 'package:gui_flutter/src/rust/api/library.dart';

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

  Future<void> _clearStems() async {
    final confirmed = await showMixarConfirm<bool>(
      context: context,
      title: 'Clear stem cache?',
      body: 'Deletes generated stem files for all tracks. Original library audio is not touched.',
      actions: const [
        MixarDialogAction(
          label: 'Cancel',
          value: false,
          variant: MixarButtonVariant.outline,
        ),
        MixarDialogAction(label: 'Clear', value: true),
      ],
    );
    if (confirmed != true || !mounted) {
      return;
    }
    await _runClear((t) => t.clearStemCache(), 'Stem cache cleared');
  }

  Future<void> _clearModels() async {
    final confirmed = await showMixarConfirm<bool>(
      context: context,
      title: 'Clear model cache?',
      body: 'Deletes downloaded stem separation models. They will re-download when needed.',
      actions: const [
        MixarDialogAction(
          label: 'Cancel',
          value: false,
          variant: MixarButtonVariant.outline,
        ),
        MixarDialogAction(label: 'Clear', value: true),
      ],
    );
    if (confirmed != true || !mounted) {
      return;
    }
    await _runClear((t) => t.clearModelCache(), 'Model cache cleared');
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

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      spacing: 16,
      children: [
        const SettingsSectionHeader(
          title: 'Storage',
          description: 'Disk use for offline stem files and separation models.',
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
            crossAxisAlignment: CrossAxisAlignment.start,
            spacing: 16,
            children: [
              _StorageRow(
                label: 'Stem cache',
                sizeLabel: _loading || usage == null
                    ? '…'
                    : formatStorageBytes(usage.stemsBytes),
                onClear: _busy || _loading ? null : _clearStems,
              ),
              _StorageRow(
                label: 'Model cache',
                sizeLabel: _loading || usage == null
                    ? '…'
                    : formatStorageBytes(usage.modelsBytes),
                onClear: _busy || _loading ? null : _clearModels,
              ),
            ],
          ),
        ),
      ],
    );
  }
}

class _StorageRow extends StatelessWidget {
  const new({
    required this.label,
    required this.sizeLabel,
    required this.onClear,
  });

  final String label;
  final String sizeLabel;
  final VoidCallback? onClear;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return Row(
      children: [
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(label, style: theme.typography.body.sm),
              const SizedBox(height: 2),
              Text(
                sizeLabel,
                style: theme.typography.body.xs.copyWith(
                  color: theme.colors.mutedForeground,
                ),
              ),
            ],
          ),
        ),
        AppButton(
          variant: MixarButtonVariant.outline,
          size: MixarButtonSize.sm,
          onPress: onClear,
          child: const Text('Clear'),
        ),
      ],
    );
  }
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
