import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';

/// Library placeholder: one panel with collections sidebar + track table.
class LibraryPanel extends StatelessWidget {
  const LibraryPanel({super.key});

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.all(8),
      child: FCard(
        child: FResizable(
          axis: .horizontal,
          divider: .dividerWithThumb,
          children: [
            FResizableRegion.flex(
              flex: 1,
              minFlex: 1,
              builder: _fill,
              child: const _CollectionsPane(),
            ),
            FResizableRegion.flex(
              flex: 2,
              minFlex: 1,
              builder: _fill,
              child: const _TrackTablePane(),
            ),
          ],
        ),
      ),
    );
  }

  static Widget _fill(BuildContext _, FResizableRegionData _, Widget? child) =>
      child!;
}

class _CollectionsPane extends StatelessWidget {
  const _CollectionsPane();

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;

    return Padding(
      padding: const EdgeInsets.all(12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            'Collections  ·  Drive',
            style: theme.typography.body.sm.copyWith(fontWeight: FontWeight.w600),
          ),
          const SizedBox(height: 12),
          Text('samples', style: theme.typography.body.sm),
          Text(
            '4 tracks (placeholder)',
            style: theme.typography.body.xs.copyWith(
              color: theme.colors.mutedForeground,
            ),
          ),
          const Spacer(),
        ],
      ),
    );
  }
}

class _TrackTablePane extends StatelessWidget {
  const _TrackTablePane();

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;

    return Padding(
      padding: const EdgeInsets.all(12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          const FTextField(hint: 'Filter tracks…'),
          const SizedBox(height: 12),
          Row(
            children: [
              for (final h in ['Title', 'Artist', 'BPM', 'Key', 'Length'])
                Expanded(
                  child: Text(
                    h,
                    style: theme.typography.body.xs.copyWith(
                      color: theme.colors.mutedForeground,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                ),
            ],
          ),
          const FDivider(),
          Expanded(
            child: Center(
              child: Text(
                'Track list placeholder',
                style: theme.typography.body.sm.copyWith(
                  color: theme.colors.mutedForeground,
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}
