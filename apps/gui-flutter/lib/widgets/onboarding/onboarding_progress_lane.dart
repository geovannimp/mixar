import 'package:flutter/widgets.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// Vertical setup track beside the headline.
///
/// A full-height rail with the walked distance painted in accent, the current
/// stop lit and the rest waiting. The fill is its own animation: advancing a
/// step slides it down instead of jumping, so the run feels like a track
/// rather than a page counter.
class OnboardingProgressLane extends StatefulWidget {
  const new({required this.step, required this.total, super.key});

  final int step;
  final int total;

  @override
  State<OnboardingProgressLane> createState() => _OnboardingProgressLaneState();
}

class _OnboardingProgressLaneState extends State<OnboardingProgressLane> {
  /// Stop pitch and marker box; the spine runs through the markers' centre.
  static const _pitch = 30.0;
  static const _marker = 10.0;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final colors = theme.colors;
    final l10n = AppLocalizations.of(context)!;
    final step = widget.step;
    final total = widget.total;
    final walk = total <= 1 ? 1.0 : step / (total - 1);
    final dim = colors.mutedForeground.withValues(alpha: 0.55);
    // The rail has to be visible against the stage, unlike a card hairline.
    final railColor = colors.mutedForeground.withValues(alpha: 0.3);

    TextStyle number(Color color, FontWeight weight) =>
        theme.typography.mono.xs.copyWith(
          color: color,
          height: 1,
          letterSpacing: 0.8,
          fontWeight: weight,
        );

    return Semantics(
      container: true,
      label: l10n.onboardingStepProgress(step + 1, total),
      child: ExcludeSemantics(
        child: Stack(
          children: [
            // Rail: the whole track, behind the stops.
            Positioned(
              left: _marker / 2 - 1,
              top: _marker / 2,
              bottom: _marker / 2,
              child: SizedBox(width: 2, child: ColoredBox(color: railColor)),
            ),
            // Walked distance: its own animation, so advancing slides.
            Positioned(
              left: _marker / 2 - 1,
              top: _marker / 2,
              bottom: _marker / 2,
              child: AnimatedFractionallySizedBox(
                duration: const Duration(milliseconds: 420),
                curve: Curves.easeOutCubic,
                alignment: Alignment.topCenter,
                heightFactor: walk,
                child: SizedBox(
                  width: 2,
                  child: ColoredBox(color: colors.accent),
                ),
              ),
            ),
            Column(
              mainAxisSize: MainAxisSize.min,
              spacing: _pitch - _marker,
              children: [
                for (var i = 0; i < total; i++)
                  Row(
                    mainAxisSize: MainAxisSize.min,
                    spacing: 9,
                    children: [
                      SizedBox.square(
                        dimension: _marker,
                        child: Center(
                          child: AnimatedContainer(
                            duration: const Duration(milliseconds: 260),
                            curve: Curves.easeOutCubic,
                            width: i == step ? _marker : 5,
                            height: i == step ? _marker : 5,
                            decoration: BoxDecoration(
                              color: i == step
                                  ? colors.accent
                                  : i < step
                                  ? colors.accent.withValues(alpha: 0.8)
                                  : colors.background,
                              // Future stops are hollow, and every stop keeps
                              // the spine readable underneath it.
                              border: i > step
                                  ? Border.all(color: railColor)
                                  : null,
                            ),
                          ),
                        ),
                      ),
                      Text(
                        (i + 1).toString().padLeft(2, '0'),
                        style: number(
                          i == step
                              ? colors.accent
                              : i < step
                              ? colors.mutedForeground
                              : dim,
                          i == step ? FontWeight.w600 : FontWeight.w400,
                        ),
                      ),
                    ],
                  ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}
