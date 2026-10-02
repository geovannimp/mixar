import 'package:flutter/widgets.dart';
import 'package:gui_flutter/settings/settings_widgets.dart';
import 'package:gui_flutter/shell/app_button.dart';
import 'package:gui_flutter/shell/mixar_dialog.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';

/// Options for collection-wide analysis, mirroring the non-force skip rule in
/// `TrackMetadata::needs_analysis` (skip tracks that already carry BPM/key or
/// have an analysis row).
class AnalyzeCollectionOptions {
  const new({required this.force, required this.generateStems});

  /// Re-analyze every file track, overriding tag BPM/key.
  final bool force;

  /// Also queue stem generation for every file track (one at a time on the
  /// serial stem queue; no-op for valid caches and native `.stem.mp4` files).
  final bool generateStems;
}

/// Options dialog for collection-wide analysis: skip-up-to-date default, force
/// re-analysis opt-in, and optional stem generation.
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
          SettingsToggle(
            label: 'Force re-analysis',
            value: _force,
            onChanged: (value) => setState(() => _force = value),
          ),
          const SizedBox(height: 8),
          SettingsToggle(
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
