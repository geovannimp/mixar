import 'dart:math' as math;

import 'package:flutter/widgets.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/widgets/onboarding/onboarding_body.dart';
import 'package:gui_flutter/widgets/onboarding/onboarding_floating_hero.dart';

export 'package:gui_flutter/widgets/onboarding/onboarding_floating_hero.dart';

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
