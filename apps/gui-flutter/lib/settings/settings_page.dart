import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/settings/settings_audio_panel.dart';
import 'package:gui_flutter/settings/settings_controllers_panel.dart';
import 'package:gui_flutter/settings/settings_deck_panel.dart';
import 'package:gui_flutter/settings/settings_library_panel.dart';
import 'package:gui_flutter/settings/settings_session_panel.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_mixer_panel.dart';
import 'package:gui_flutter/settings/settings_providers.dart';
import 'package:gui_flutter/settings/settings_section.dart';
import 'package:gui_flutter/settings/settings_sidebar.dart';
import 'package:gui_flutter/settings/settings_waveform_panel.dart';
import 'package:gui_flutter/shell/controller_providers.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';

class SettingsPage extends ConsumerStatefulWidget {
  const SettingsPage({this.onClose, super.key});

  final VoidCallback? onClose;

  @override
  ConsumerState<SettingsPage> createState() => _SettingsPageState();
}

class _SettingsPageState extends ConsumerState<SettingsPage> {
  var _section = SettingsSection.audio;
  AppSettings? _draft;
  AppSettings? _baseline;
  var _busy = false;
  String? _error;
  var _saved = false;

  bool _isDirty(AppSettings draft, AppSettings baseline) =>
      appSettingsDirty(draft, baseline);

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final settingsAsync = ref.watch(appSettingsProvider);

    return settingsAsync.when(
      loading: () => Center(
        child: Text(
          'Loading settings…',
          style: theme.typography.body.sm.copyWith(
            color: theme.colors.mutedForeground,
          ),
        ),
      ),
      error: (e, _) => Center(
        child: Text(
          '$e',
          style: theme.typography.body.sm.copyWith(
            color: theme.colors.destructive,
          ),
        ),
      ),
      data: (settings) {
        _baseline ??= settings;
        _draft ??= settings;
        final draft = _draft!;
        final baseline = _baseline!;
        final dirty = _isDirty(draft, baseline);
        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            DecoratedBox(
              decoration: BoxDecoration(
                border: Border(bottom: BorderSide(color: theme.colors.border)),
              ),
              child: Padding(
                padding: const EdgeInsets.fromLTRB(12, 12, 24, 12),
                child: Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  spacing: 12,
                  children: [
                    if (widget.onClose != null)
                      FButton.icon(
                        variant: .ghost,
                        size: .sm,
                        semanticsLabel: 'Close',
                        onPress: _busy ? null : () => _close(dirty),
                        child: const Icon(FLucideIcons.x),
                      ),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            'SETTINGS',
                            style: theme.typography.body.xs.copyWith(
                              fontWeight: FontWeight.w700,
                              letterSpacing: 2,
                            ),
                          ),
                          const SizedBox(height: 4),
                          Text(
                            'Saving restarts the engine automatically if it\'s running.',
                            style: theme.typography.body.sm.copyWith(
                              color: theme.colors.mutedForeground,
                            ),
                          ),
                          if (_error != null) ...[
                            const SizedBox(height: 8),
                            Text(
                              _error!,
                              style: theme.typography.body.sm.copyWith(
                                color: theme.colors.destructive,
                              ),
                            ),
                          ],
                          if (_saved && _error == null) ...[
                            const SizedBox(height: 8),
                            Text(
                              'Settings saved.',
                              style: theme.typography.body.sm.copyWith(
                                color: theme.colors.primary,
                              ),
                            ),
                          ],
                        ],
                      ),
                    ),
                    if (dirty)
                      FButton(
                        size: .sm,
                        mainAxisSize: .min,
                        onPress: _busy ? null : () => _save(draft),
                        child: Text(_busy ? 'Saving…' : 'Save'),
                      ),
                  ],
                ),
              ),
            ),
            Expanded(
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  SettingsSidebar(
                    active: _section,
                    onSelect: (section) => setState(() => _section = section),
                  ),
                  Expanded(
                    child: SingleChildScrollView(
                      padding: const EdgeInsets.all(24),
                      child: ConstrainedBox(
                        constraints: const BoxConstraints(maxWidth: 672),
                        child: _SettingsSectionPanel(
                          section: _section,
                          draft: draft,
                          onChanged: (next) => setState(() {
                            _draft = next;
                            _saved = false;
                          }),
                        ),
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ],
        );
      },
    );
  }

  Future<void> _save(AppSettings draft) async {
    setState(() {
      _busy = true;
      _error = null;
      _saved = false;
    });
    try {
      final result = await saveAppSettings(ref, draft);
      if (!mounted) {
        return;
      }
      if (result.trustedControllersChanged) {
        ref.invalidate(controllerTransportProvider);
        ref.invalidate(controllerMappingsProvider);
        ref.invalidate(controllerDevicesProvider);
      }
      setState(() {
        _draft = result.saved;
        _baseline = result.saved;
        _saved = result.applyError == null;
        _error = result.applyError;
      });
    } catch (e) {
      if (mounted) {
        setState(() => _error = '$e');
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  Future<void> _close(bool dirty) async {
    if (!dirty) {
      widget.onClose?.call();
      return;
    }
    final choice = await showFDialog<_CloseChoice>(
      context: context,
      builder: (context, _, animation) {
        return FDialog(
          animation: animation,
          builder: (context, _) {
            final theme = context.theme;
            return Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                mainAxisSize: .min,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Text(
                    'Unsaved settings',
                    style: theme.typography.body.md.copyWith(
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                  const SizedBox(height: 8),
                  const Text('Save changes before closing?'),
                  const SizedBox(height: 16),
                  Row(
                    spacing: 8,
                    children: [
                      FButton(
                        variant: .outline,
                        size: .sm,
                        onPress: () =>
                            Navigator.of(context).pop(_CloseChoice.cancel),
                        child: const Text('Cancel'),
                      ),
                      FButton(
                        variant: .ghost,
                        size: .sm,
                        onPress: () =>
                            Navigator.of(context).pop(_CloseChoice.discard),
                        child: const Text('Discard'),
                      ),
                      FButton(
                        size: .sm,
                        onPress: () =>
                            Navigator.of(context).pop(_CloseChoice.save),
                        child: const Text('Save'),
                      ),
                    ],
                  ),
                ],
              ),
            );
          },
        );
      },
    );
    if (!mounted) {
      return;
    }
    switch (choice) {
      case null:
      case _CloseChoice.cancel:
        return;
      case _CloseChoice.discard:
        widget.onClose?.call();
        return;
      case _CloseChoice.save:
        final draft = _draft;
        if (draft == null) {
          return;
        }
        await _save(draft);
        if (mounted && _error == null) {
          widget.onClose?.call();
        }
    }
  }
}

enum _CloseChoice { save, discard, cancel }

class _SettingsSectionPanel extends StatelessWidget {
  const _SettingsSectionPanel({
    required this.section,
    required this.draft,
    required this.onChanged,
  });

  final SettingsSection section;
  final AppSettings draft;
  final ValueChanged<AppSettings> onChanged;

  @override
  Widget build(BuildContext context) {
    return switch (section) {
      SettingsSection.audio => SettingsAudioPanel(
        draft: draft,
        onChanged: onChanged,
      ),
      SettingsSection.mixer => SettingsMixerPanel(
        draft: draft,
        onChanged: onChanged,
      ),
      SettingsSection.waveform => SettingsWaveformPanel(
        draft: draft,
        onChanged: onChanged,
      ),
      SettingsSection.deck => SettingsDeckPanel(
        draft: draft,
        onChanged: onChanged,
      ),
      SettingsSection.library => SettingsLibraryPanel(
        draft: draft,
        onChanged: onChanged,
      ),
      SettingsSection.session => SettingsSessionPanel(
        draft: draft,
        onChanged: onChanged,
      ),
      SettingsSection.controllers => SettingsControllersPanel(
        draft: draft,
        onChanged: onChanged,
      ),
    };
  }
}
