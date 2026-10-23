import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/settings/settings_checkbox.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/mixar_dialog.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// Options for collection-wide analysis. Re-analysis is also the explicit
/// opt-in to replace existing BPM/key and beat grid.
class AnalyzeCollectionOptions {
  const new({required this.force, required this.generateStems});

  /// Re-analyze every file track and replace existing BPM/key and beat grid.
  final bool force;

  /// Also queue stem generation for every file track (one at a time on the
  /// serial stem queue; no-op for valid caches and native `.stem.mp4` files).
  final bool generateStems;
}

/// Options dialog for collection-wide analysis: skip-up-to-date default, opt-in
/// replacement of existing analysis, and optional stem generation.
Future<AnalyzeCollectionOptions?> showAnalyzeCollectionDialog(
  BuildContext context, {
  required String collectionName,
  required int trackCount,
}) {
  return showMixarDialog<AnalyzeCollectionOptions?>(
    context: context,
    builder: (context) => _AnalyzeCollectionDialogBody(
      collectionName: collectionName,
      trackCount: trackCount,
    ),
  );
}

class _AnalyzeCollectionDialogBody extends StatefulWidget {
  const new({required this.collectionName, required this.trackCount});

  final String collectionName;
  final int trackCount;

  @override
  State<_AnalyzeCollectionDialogBody> createState() =>
      _AnalyzeCollectionDialogBodyState();
}

class _AnalyzeCollectionDialogBodyState
    extends State<_AnalyzeCollectionDialogBody> {
  var _force = false;
  var _generateStems = false;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return Padding(
      padding: const EdgeInsets.all(16),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            'Analyze ${widget.collectionName}',
            style: theme.typography.body.md.copyWith(
              fontWeight: FontWeight.w700,
            ),
          ),
          const SizedBox(height: 8),
          Text(
            '${widget.trackCount} tracks. Tracks that already have BPM/key '
            'or were analyzed before are skipped.',
            style: theme.typography.body.sm.copyWith(
              color: theme.colors.mutedForeground,
            ),
          ),
          const SizedBox(height: 16),
          SettingsCheckbox(
            label:
                'Force re-analysis — replaces existing BPM/key and beat grid',
            value: _force,
            onChanged: (value) => setState(() => _force = value),
          ),
          const SizedBox(height: 8),
          SettingsCheckbox(
            label: 'Also generate stems',
            value: _generateStems,
            onChanged: (value) => setState(() => _generateStems = value),
          ),
          const SizedBox(height: 16),
          Row(
            spacing: 8,
            children: [
              AppButton(
                variant: .outline,
                onPress: () => Navigator.of(context).pop(),
                child: const Text('Cancel'),
              ),
              AppButton(
                onPress: () {
                  Navigator.of(context).pop(
                    AnalyzeCollectionOptions(
                      force: _force,
                      generateStems: _generateStems,
                    ),
                  );
                },
                child: const Text('Start analysis'),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

/// Per-track analysis confirmation. Existing BPM/key and beat grid are kept
/// unless the user checks the replacement option.
Future<bool?> showAnalyzeTrackDialog(
  BuildContext context, {
  required String trackName,
}) {
  return showMixarDialog<bool?>(
    context: context,
    builder: (context) => _AnalyzeTrackDialogBody(trackName: trackName),
  );
}

class _AnalyzeTrackDialogBody extends StatefulWidget {
  const _AnalyzeTrackDialogBody({required this.trackName});

  final String trackName;

  @override
  State<_AnalyzeTrackDialogBody> createState() =>
      _AnalyzeTrackDialogBodyState();
}

class _AnalyzeTrackDialogBodyState extends State<_AnalyzeTrackDialogBody> {
  var _force = false;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return Padding(
      padding: const EdgeInsets.all(16),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            'Analyze ${widget.trackName}',
            style: theme.typography.body.md.copyWith(
              fontWeight: FontWeight.w700,
            ),
          ),
          const SizedBox(height: 8),
          Text(
            'Existing BPM/key are kept by default; analysis fills missing '
            'values only.',
            style: theme.typography.body.sm.copyWith(
              color: theme.colors.mutedForeground,
            ),
          ),
          const SizedBox(height: 16),
          SettingsCheckbox(
            label: 'Replace existing BPM/key and beat grid',
            value: _force,
            onChanged: (value) => setState(() => _force = value),
          ),
          const SizedBox(height: 16),
          Row(
            spacing: 8,
            children: [
              AppButton(
                variant: .outline,
                onPress: () => Navigator.of(context).pop(),
                child: const Text('Cancel'),
              ),
              AppButton(
                onPress: () => Navigator.of(context).pop(_force),
                child: const Text('Start analysis'),
              ),
            ],
          ),
        ],
      ),
    );
  }
}
