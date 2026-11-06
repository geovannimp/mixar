import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/theme/mixar_button_style.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/src/rust/api/controller.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/onboarding/onboarding_ui.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Step 3: trust a detected controller so it connects on its own from now on.
///
/// Only shown when the host reported at least one MIDI input; [controllers] is
/// never empty here. Trust is a real settings write
/// (`trustedControllerDeviceIds`), which the mixer uses when it starts the
/// controller host.
class OnboardingControllersStep extends StatelessWidget {
  const new({
    required this.draft,
    required this.controllers,
    required this.onChanged,
    super.key,
  });

  final AppSettings draft;
  final List<DetectedController> controllers;
  final ValueChanged<AppSettings> onChanged;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final theme = context.theme;
    return OnboardingBody(
      eyebrow: l10n.onboardingKickerControllers,
      title: l10n.onboardingControllersTitle,
      description: l10n.onboardingControllersDetected(controllers.length),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        spacing: 12,
        children: [
          for (final controller in controllers)
            _ControllerCard(
              controller: controller,
              trusted: _isTrusted(controller),
              onChanged: (trusted) =>
                  onChanged(_withTrust(controller, trusted)),
            ),
          Padding(
            padding: const EdgeInsets.only(top: 6),
            child: Text(
              l10n.onboardingControllersMappings,
              style: theme.typography.body.sm.copyWith(
                color: theme.colors.mutedForeground,
              ),
            ),
          ),
        ],
      ),
    );
  }

  bool _isTrusted(DetectedController controller) {
    final deviceId = controller.deviceId;
    return deviceId != null &&
        draft.trustedControllerDeviceIds.contains(deviceId);
  }

  AppSettings _withTrust(DetectedController controller, bool trusted) {
    final deviceId = controller.deviceId;
    if (deviceId == null) {
      return draft;
    }
    final ids = [...draft.trustedControllerDeviceIds]
      ..removeWhere((id) => id == deviceId);
    if (trusted) {
      ids.add(deviceId);
    }
    return copyAppSettings(draft, trustedControllerDeviceIds: ids);
  }
}

/// One detected port: name, the mapping it will get, and the trust toggle.
class _ControllerCard extends StatelessWidget {
  const new({
    required this.controller,
    required this.trusted,
    required this.onChanged,
  });

  final DetectedController controller;
  final bool trusted;
  final ValueChanged<bool> onChanged;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final l10n = AppLocalizations.of(context)!;
    // The raw ALSA port name ("DDJ-400:DDJ-400 MIDI 1 36:0") means nothing to
    // a person; the mapping's display name is the device they own.
    final name = controller.mappingName ?? controller.portName;
    final canTrust = controller.deviceId != null;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: theme.colors.card,
        border: Border.all(
          color: trusted ? theme.colors.primary : theme.colors.border,
          width: trusted ? 2 : theme.style.borderWidth,
        ),
      ),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          spacing: 16,
          children: [
            Icon(
              LucideIcons.usb,
              size: 20,
              color: trusted
                  ? theme.colors.primary
                  : theme.colors.mutedForeground,
            ),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                spacing: 4,
                children: [
                  Text(
                    name,
                    style: theme.typography.body.lg.copyWith(
                      fontWeight: FontWeight.w600,
                      height: 1.25,
                    ),
                  ),
                ],
              ),
            ),
            if (canTrust)
              AppButton(
                variant: trusted
                    ? MixarButtonVariant.secondary
                    : MixarButtonVariant.outline,
                size: MixarButtonSize.sm,
                onPress: () => onChanged(!trusted),
                child: Text(
                  trusted
                      ? l10n.onboardingControllersTrustedAction
                      : l10n.onboardingControllersTrust,
                ),
              ),
          ],
        ),
      ),
    );
  }
}
