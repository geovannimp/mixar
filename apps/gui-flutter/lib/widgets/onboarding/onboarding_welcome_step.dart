import 'dart:math' as math;

import 'package:flutter/widgets.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/widgets/onboarding/onboarding_backdrop.dart';
import 'package:gui_flutter/widgets/onboarding/onboarding_ui.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Step 1: what Mixar is, as a drifting word cloud rather than a form.
class OnboardingWelcomeStep extends StatelessWidget {
  const new({required this.appTitle, super.key});

  /// The product's display name, highlighted inside the localised headline.
  final String appTitle;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return OnboardingBody(
      emphasis: true,
      eyebrow: l10n.onboardingKickerSetup,
      title: l10n.onboardingWelcomeTitle,
      titleHighlight: appTitle,
      description: l10n.onboardingWelcomeBody,
      // A bounded stage: the drifting Stack needs finite bounds, and the box
      // has to shrink with the window instead of forcing 560 px.
      child: Center(
        child: LayoutBuilder(
          builder: (_, constraints) {
            final width = math.min(560, constraints.maxWidth).toDouble();
            return SizedBox(
              width: width,
              height: math.min(420, width / 1.3),
              // Decoration only: fourteen labels announced before the headline
              // would bury the step's real copy.
              child: const ExcludeSemantics(child: OnboardingFloatingHero()),
            );
          },
        ),
      ),
    );
  }
}

/// The marketing-site hero shape: icons and words floating in space.
///
/// Positions are hand-tuned to the reachable box so nothing collides at any
/// window size; the drift only shifts each item a few pixels, and it stops
/// entirely when the system asks for reduced motion. Every item is drawn in
/// the accent so the cloud reads as one object rather than two styles.
class OnboardingFloatingHero extends StatefulWidget {
  const new({super.key});

  @override
  State<OnboardingFloatingHero> createState() => _OnboardingFloatingHeroState();
}

class _OnboardingFloatingHeroState extends State<OnboardingFloatingHero>
    with SingleTickerProviderStateMixin {
  late final AnimationController _controller = AnimationController(
    vsync: this,
    duration: const Duration(seconds: 18),
  );

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (onboardingMotionDisabled(context)) {
      _controller.stop();
    } else if (!_controller.isAnimating) {
      _controller.repeat();
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    // Align (-1..1 on each axis), drift amplitude in pixels, phase, type size.
    final items = <_HeroItem>[
      _HeroItem(
        x: -0.95,
        y: -0.78,
        amp: 7,
        phase: 0,
        icon: LucideIcons.layers2,
        label: l10n.onboardingFloatDecks,
        size: 28,
      ),
      _HeroItem(
        x: 0.1,
        y: -0.96,
        amp: 9,
        phase: 0.3,
        icon: LucideIcons.audioWaveform,
        label: l10n.onboardingFloatWaveform,
        size: 14,
      ),
      _HeroItem(
        x: -0.34,
        y: -0.44,
        amp: 6,
        phase: 0.62,
        icon: LucideIcons.gitBranch,
        label: l10n.onboardingFloatStems,
        size: 22,
      ),
      _HeroItem(
        x: -0.86,
        y: -0.04,
        amp: 8,
        phase: 0.15,
        icon: LucideIcons.slidersHorizontal,
        label: l10n.onboardingFloatMixer,
        size: 14,
      ),
      _HeroItem(
        x: 0.95,
        y: -0.3,
        amp: 7,
        phase: 0.5,
        icon: LucideIcons.disc3,
        label: l10n.onboardingFloatJog,
        size: 14,
      ),
      _HeroItem(
        x: -0.52,
        y: 0.28,
        amp: 10,
        phase: 0.8,
        icon: LucideIcons.refreshCw,
        label: l10n.onboardingFloatSync,
        size: 24,
      ),
      _HeroItem(
        x: -0.96,
        y: 0.72,
        amp: 6,
        phase: 0.42,
        icon: LucideIcons.headphones,
        label: l10n.onboardingFloatCue,
        size: 14,
      ),
      _HeroItem(
        x: 0.36,
        y: 0.44,
        amp: 8,
        phase: 0.05,
        icon: LucideIcons.repeat,
        label: l10n.onboardingFloatLoop,
        size: 20,
      ),
      _HeroItem(
        x: -0.16,
        y: 0.96,
        amp: 6,
        phase: 0.72,
        icon: LucideIcons.zap,
        label: l10n.onboardingFloatFx,
        size: 14,
      ),
      _HeroItem(
        x: 0.78,
        y: 0.86,
        amp: 5,
        phase: 0.22,
        icon: LucideIcons.music,
        label: l10n.onboardingFloatLibrary,
        size: 14,
      ),
      // Second ring: fills the middle of the stage so the cloud reads as a
      // field rather than a border with a hole in it.
      _HeroItem(
        x: 0.48,
        y: -0.6,
        amp: 8,
        phase: 0.36,
        icon: LucideIcons.gauge,
        label: l10n.onboardingFloatBpm,
        size: 14,
      ),
      _HeroItem(
        x: -0.04,
        y: -0.02,
        amp: 11,
        phase: 0.66,
        icon: LucideIcons.music2,
        label: l10n.onboardingFloatKeys,
        size: 22,
      ),
      _HeroItem(
        x: 0.6,
        y: 0.16,
        amp: 7,
        phase: 0.12,
        icon: LucideIcons.flag,
        label: l10n.onboardingFloatCues,
        size: 14,
      ),
      _HeroItem(
        x: 0.04,
        y: 0.68,
        amp: 9,
        phase: 0.52,
        icon: LucideIcons.activity,
        label: l10n.onboardingFloatAnalyze,
        size: 14,
      ),
    ];

    return RepaintBoundary(
      child: AnimatedBuilder(
        animation: _controller,
        builder: (animationContext, _) => LayoutBuilder(
          builder: (layoutContext, constraints) {
            final t = _controller.value * math.pi * 2;
            return Stack(
              clipBehavior: Clip.none,
              children: [
                for (final item in items)
                  // Align, not Positioned: a long word near an edge is kept
                  // inside the box instead of painting past it.
                  Align(
                    alignment: Alignment(item.x, item.y),
                    child: Transform.translate(
                      offset: Offset(
                        0,
                        math.sin(t + item.phase * math.pi * 2) * item.amp,
                      ),
                      child: Transform.rotate(
                        angle: math.sin(t * 0.5 + item.phase) * 0.015,
                        child: _HeroItemView(item: item),
                      ),
                    ),
                  ),
              ],
            );
          },
        ),
      ),
    );
  }
}

class _HeroItem {
  const new({
    required this.x,
    required this.y,
    required this.amp,
    required this.phase,
    required this.icon,
    required this.label,
    required this.size,
  });

  final double x;
  final double y;
  final double amp;
  final double phase;
  final IconData icon;
  final String label;

  /// Label size: the big words carry the cloud, the rest are chips of text.
  final double size;
}

/// Icon + word, both in the accent, sized by the item.
class _HeroItemView extends StatelessWidget {
  const new({required this.item});

  final _HeroItem item;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final colors = theme.colors;
    final large = item.size >= 20;
    return Row(
      mainAxisSize: MainAxisSize.min,
      spacing: large ? 10 : 7,
      children: [
        Icon(
          item.icon,
          size: large ? item.size * 0.72 : 14,
          color: colors.accent,
        ),
        Text(
          item.label,
          style: large
              ? theme.typography.display.xl3.copyWith(
                  fontSize: item.size,
                  height: 1,
                  letterSpacing: -0.6,
                  fontWeight: FontWeight.w600,
                  color: colors.foreground,
                )
              : theme.typography.body.sm.copyWith(
                  color: colors.foreground,
                  letterSpacing: 0.2,
                ),
        ),
      ],
    );
  }
}
