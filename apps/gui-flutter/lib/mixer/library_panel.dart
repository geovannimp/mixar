import 'package:flutter/foundation.dart' show kIsWeb;
import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/library/collections_pane.dart';
import 'package:gui_flutter/library/track_table_pane.dart';

/// Library panel: collections sidebar + track table.
class LibraryPanel extends StatelessWidget {
  const LibraryPanel({super.key});

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return Padding(
      padding: const EdgeInsets.all(8),
      child: FCard(
        clipBehavior: Clip.antiAlias,
        child: kIsWeb
            ? Center(
                child: Text(
                  'Library browse is desktop-only',
                  style: theme.typography.body.sm.copyWith(
                    color: theme.colors.mutedForeground,
                  ),
                ),
              )
            : FResizable(
                axis: .horizontal,
                divider: .dividerWithThumb,
                children: [
                  .fixed(
                    minExtent: 100,
                    extent: 200,
                    builder: _fill,
                    child: const CollectionsPane(),
                  ),
                  .flex(
                    flex: 3,
                    minFlex: 1,
                    builder: _fill,
                    child: const TrackTablePane(),
                  ),
                ],
              ),
      ),
    );
  }

  static Widget _fill(BuildContext _, FResizableRegionData _, Widget? child) =>
      child!;
}
