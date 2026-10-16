import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/utils/library_list_layout.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/widgets/library/meta_pill.dart';
import 'package:gui_flutter/widgets/shell/m_loader.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// In-flow status chips for a track's analysis / stem jobs.
///
/// Watches progress independently so a tick rebuilds the pills without
/// regenerating the row. [IgnorePointer] keeps selection and drag-to-deck on
/// the row; menus stay on the actions slot.
class TrackStatusPills extends ConsumerWidget {
  const new({required this.trackId});

  final String trackId;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final jobs =
        ref.watch(trackProgressProvider.select((byId) => byId[trackId])) ??
        const <TrackProgressInfo>[];
    final analyzing = ref.watch(
      analyzingTrackIdsProvider.select((ids) => ids.contains(trackId)),
    );
    final stemsGenerating = ref.watch(
      stemGeneratingTrackIdsProvider.select((ids) => ids.contains(trackId)),
    );
    final infos = trackStatusInfos(
      jobs: jobs,
      analyzing: analyzing,
      stemsGenerating: stemsGenerating,
    );
    if (infos.isEmpty) {
      return const SizedBox.shrink();
    }
    // LayoutBuilder + per-pill max: ellipsize under pressure without a
    // Flexible parent that would expand the group to the details width and
    // leave the label stranded left of TrailingMeta.
    return IgnorePointer(
      child: LayoutBuilder(
        builder: (context, constraints) {
          // Budget the inner Row after the trailing gap padding; otherwise two
          // pills sized to the outer max overflow by exactly [kMetaPillGap].
          final innerMax = constraints.maxWidth.isFinite
              ? (constraints.maxWidth - kMetaPillGap).clamp(
                  0.0,
                  double.infinity,
                )
              : double.infinity;
          final gapTotal = kMetaPillGap * (infos.length - 1);
          final perPillMax = infos.length == 1
              ? innerMax
              : ((innerMax - gapTotal) / infos.length).clamp(
                  0.0,
                  double.infinity,
                );
          return Padding(
            padding: const EdgeInsets.only(right: kMetaPillGap),
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                for (var i = 0; i < infos.length; i++) ...[
                  if (i > 0) const SizedBox(width: kMetaPillGap),
                  ConstrainedBox(
                    constraints: BoxConstraints(maxWidth: perPillMax),
                    child: _TrackStatusPill(info: infos[i]),
                  ),
                ],
              ],
            ),
          );
        },
      ),
    );
  }
}

class _TrackStatusPill extends StatelessWidget {
  const new({required this.info});

  final TrackProgressInfo info;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return Semantics(
      label: info.label,
      container: true,
      // Announce the phase once instead of also reading the spinner + text.
      excludeSemantics: true,
      child: MetaPill(
        text: info.label,
        fillWidth: false,
        leading: info.fraction == null
            ? MLoader(size: MLoaderSize.xs, color: theme.colors.mutedForeground)
            : null,
      ),
    );
  }
}
