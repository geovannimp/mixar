import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/library/providers.dart';

Future<void> showTrackDetailDialog(
  BuildContext context,
  WidgetRef ref, {
  required String trackId,
}) async {
  final transport = await ref.read(libraryTransportProvider.future);
  final track = await transport.getTrack(trackId: trackId);
  if (!context.mounted || track == null) {
    return;
  }
  var isrc = track.isrc ?? '';
  await showFDialog<void>(
    context: context,
    builder: (context, _, animation) {
      return FDialog(
        animation: animation,
        builder: (context, _) {
          final theme = context.theme;
          final title = track.title ?? track.displayName;
          return Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Text(
                  title,
                  style: theme.typography.body.md.copyWith(
                    fontWeight: FontWeight.w700,
                  ),
                ),
                if (track.artist != null && track.artist!.isNotEmpty) ...[
                  const SizedBox(height: 4),
                  Text(
                    track.artist!,
                    style: theme.typography.body.sm.copyWith(
                      color: theme.colors.mutedForeground,
                    ),
                  ),
                ],
                const SizedBox(height: 16),
                FTextField(
                  label: const Text('ISRC'),
                  hint: 'International Standard Recording Code',
                  control: .managed(
                    initial: TextEditingValue(text: isrc),
                    onChange: (v) => isrc = v.text,
                  ),
                ),
                const SizedBox(height: 16),
                Row(
                  spacing: 8,
                  children: [
                    FButton(
                      variant: .outline,
                      onPress: () => Navigator.of(context).pop(),
                      child: const Text('Cancel'),
                    ),
                    FButton(
                      onPress: () async {
                        try {
                          await transport.updateTrackIsrc(
                            trackId: trackId,
                            isrc: isrc.trim().isEmpty ? null : isrc.trim(),
                          );
                          ref.invalidate(collectionTracksProvider);
                          ref.invalidate(driveResolvedByPathProvider);
                          if (context.mounted) {
                            Navigator.of(context).pop();
                          }
                        } catch (e) {
                          ref
                              .read(libraryMessageProvider.notifier)
                              .setError('$e');
                        }
                      },
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
}
