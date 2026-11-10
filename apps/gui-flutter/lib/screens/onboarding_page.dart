import 'dart:async';

import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/theme/mixar_button_style.dart';
import 'package:gui_flutter/core/utils/app_assets.dart';
import 'package:gui_flutter/core/utils/desktop.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/providers/onboarding/onboarding_providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/providers/shell/controller_providers.dart';
import 'package:gui_flutter/src/rust/api/controller.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/onboarding/onboarding_appearance_step.dart';
import 'package:gui_flutter/widgets/onboarding/onboarding_backdrop.dart';
import 'package:gui_flutter/widgets/onboarding/onboarding_controllers_step.dart';
import 'package:gui_flutter/widgets/onboarding/onboarding_language_step.dart';
import 'package:gui_flutter/widgets/onboarding/onboarding_library_step.dart';
import 'package:gui_flutter/widgets/onboarding/onboarding_privacy_step.dart';
import 'package:gui_flutter/widgets/onboarding/onboarding_ready_step.dart';
import 'package:gui_flutter/widgets/onboarding/onboarding_ui.dart';
import 'package:gui_flutter/widgets/onboarding/onboarding_welcome_step.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/m_loader.dart';
import 'package:gui_flutter/widgets/shell/m_tappable.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:gui_flutter/widgets/shell/mixar_toast.dart';
import 'package:gui_flutter/widgets/shell/window_title_bar_controls.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:window_manager/window_manager.dart';

/// One page of the setup run, in order. The controller page is only in the
/// list when the host reported a MIDI input.
enum OnboardingStep {
  welcome,
  language,
  appearance,
  controllers,
  library,
  privacy,
  ready,
}

/// The run's pages, minus the ones that do not apply.
List<OnboardingStep> onboardingSteps({required bool hasController}) => [
  OnboardingStep.welcome,
  OnboardingStep.language,
  OnboardingStep.appearance,
  if (hasController) OnboardingStep.controllers,
  OnboardingStep.library,
  OnboardingStep.privacy,
  OnboardingStep.ready,
];

/// Keys the widget tests drive the flow with.
const ValueKey<String> kOnboardingNextKey = ValueKey('onboarding-next');
const ValueKey<String> kOnboardingBackKey = ValueKey('onboarding-back');
const ValueKey<String> kOnboardingSkipKey = ValueKey('onboarding-skip');

/// First-run setup: a full-window stage that replaces the mixer until setup is
/// finished or skipped.
///
/// Step edits stay in a local draft and are published to
/// `onboardingDraftProvider`, so the window itself previews the chosen theme,
/// accent and language. Only Finish writes them; Skip marks setup done and
/// drops them, which is what a user clicking past the flow expects.
class OnboardingPage extends ConsumerStatefulWidget {
  const new({required this.appTitle, super.key});

  final String appTitle;

  @override
  ConsumerState<OnboardingPage> createState() => _OnboardingPageState();
}

class _OnboardingPageState extends ConsumerState<OnboardingPage> {
  int _index = 0;
  AppSettings? _draft;
  List<DetectedController> _controllers = const [];
  List<OnboardingStep> _steps = onboardingSteps(hasController: false);
  bool _busy = false;
  final _keyboardFocus = FocusNode(debugLabel: 'onboarding');

  @override
  void initState() {
    super.initState();
    unawaited(_seed());
  }

  @override
  void dispose() {
    _keyboardFocus.dispose();
    super.dispose();
  }

  Future<void> _seed() async {
    AppSettings settings;
    try {
      settings = await ref.read(appSettingsProvider.future);
    } on Object catch (error) {
      // An unreadable settings file must not trap setup on a spinner. Saving
      // still re-reads the same provider, so this fallback cannot overwrite
      // whatever is on disk: the save fails with the toast instead.
      debugPrint('mixar: setup could not read settings: $error');
      settings = defaultAppSettings();
    }
    // The read above can resume after the page is gone; every call below uses
    // `ref`, which dies with the element.
    if (!mounted) {
      return;
    }
    // Replay re-runs the probe: the provider keeps its first answer, and the
    // hardware may have changed since.
    ref.invalidate(onboardingControllersProvider);
    // A failed MIDI probe means "no controller", never a blocked setup.
    // Only a controller Mixar has a mapping for gets a setup step: a bare MIDI
    // port (or the ALSA "Midi Through" virtual device) would be a dead row.
    final detected = await ref
        .read(onboardingControllersProvider.future)
        .catchError((Object _) => const <DetectedController>[]);
    // One row per device: a controller that exposes several MIDI ports is
    // still one controller, and trust is stored per device id anyway.
    // A row without a device id could not be trusted, so it would be a dead
    // card here; the host reports an id for every mapped bundle it returns.
    final byDevice = <String, DetectedController>{};
    for (final controller in detected) {
      final deviceId = controller.deviceId;
      if (deviceId != null) {
        byDevice.putIfAbsent(deviceId, () => controller);
      }
    }
    final controllers = byDevice.values.toList();
    if (!mounted) {
      return;
    }
    setState(() {
      _draft = settings;
      _controllers = controllers;
      _steps = onboardingSteps(hasController: controllers.isNotEmpty);
    });
    ref.read(onboardingDraftProvider.notifier).set(settings);
    _keyboardFocus.requestFocus();
  }

  void _apply(AppSettings next) {
    setState(() => _draft = next);
    ref.read(onboardingDraftProvider.notifier).set(next);
  }

  bool get _last => _index == _steps.length - 1;

  void _next() {
    if (_last) {
      unawaited(_finish());
      return;
    }
    setState(() => _index++);
  }

  void _back() {
    if (_index == 0) {
      return;
    }
    setState(() => _index--);
  }

  Future<void> _finish() async {
    final result = await _save(_draft!);
    if (result == null || !mounted) {
      // No page left to preview on or toast on: the mixer is already up.
      return;
    }
    // Hand the saved settings to the preview so the mixer swaps in on the
    // theme setup just used instead of flashing the previous one.
    ref.read(onboardingDraftProvider.notifier).set(result.saved);
    _reportOutcome(result);
  }

  void _reportSaveFailure(Object error) {
    showMixarToast(
      context: context,
      title: Text(AppLocalizations.of(context)!.onboardingSaveFailedToast),
      description: Text('$error'),
      variant: MixarToastVariant.destructive,
    );
  }

  /// The apply warning Settings shows when a kept-alive save failed to reach
  /// the running library or engine.
  void _reportOutcome(SaveAppSettingsResult result) {
    final error = result.applyError;
    if (error == null) {
      return;
    }
    showMixarToast(
      context: context,
      title: Text(AppLocalizations.of(context)!.settingsSavedNotAppliedToast),
      description: Text(error),
      variant: MixarToastVariant.destructive,
    );
  }

  Future<void> _skip() async {
    if (_busy) {
      return;
    }
    // Lock the page before the first await: the settings read can take a
    // frame, and a second click would enter `_save` twice.
    setState(() => _busy = true);
    // Skip means "not now": persist the saved settings, not the step edits.
    final AppSettings saved;
    try {
      saved = await ref.read(appSettingsProvider.future);
    } on Object catch (e) {
      // Skip keeps what is already saved; without a readable save there is
      // nothing to keep. Report it instead of failing with an unhandled error.
      if (mounted) {
        setState(() => _busy = false);
        _reportSaveFailure(e);
      }
      return;
    }
    // The read above can resume after the page is gone (fast skip, or a slow
    // settings load), and `_save` opens with `setState`.
    if (!mounted) {
      return;
    }
    final result = await _save(saved);
    if (!mounted) {
      return;
    }
    // Skip keeps no step edits, but the save it makes can still fail to apply.
    if (result != null) {
      _reportOutcome(result);
    }
  }

  /// Persist [settings] with setup marked done. Returns `null` on failure.
  ///
  /// The save's own invalidation can swap `home` for the mixer mid-flight,
  /// which kills this page's `ref`. Everything after that point therefore runs
  /// on a container captured while the page is still mounted.
  Future<SaveAppSettingsResult?> _save(AppSettings settings) async {
    final container = ProviderScope.containerOf(context, listen: false);
    setState(() => _busy = true);
    final SaveAppSettingsResult result;
    try {
      result = await saveAppSettings(
        ref,
        copyAppSettings(settings, onboardingCompleted: true),
        container: container,
      );
    } on Object catch (e) {
      if (mounted) {
        // Back/Next/Skip and the keyboard all read `_busy`: leaving it set
        // after a failure would lock the page until a restart.
        setState(() => _busy = false);
        _reportSaveFailure(e);
      }
      return null;
    }
    if (result.trustedControllersChanged) {
      // A running controller host keeps the old trust list until these refresh,
      // and it has to happen even when this page is already gone.
      container
        ..invalidate(controllerTransportProvider)
        ..invalidate(controllerMappingsProvider)
        ..invalidate(controllerDevicesProvider);
    }
    if (mounted) {
      setState(() => _busy = false);
    }
    return result;
  }

  /// Enter advances, arrows walk the flow. Buttons keep their own focus order;
  /// this only fires when no control has taken the key.
  KeyEventResult _onKey(FocusNode node, KeyEvent event) {
    if (event is! KeyDownEvent || _busy || _draft == null) {
      return KeyEventResult.ignored;
    }
    final key = event.logicalKey;
    if (key == LogicalKeyboardKey.enter ||
        key == LogicalKeyboardKey.numpadEnter ||
        key == LogicalKeyboardKey.arrowRight) {
      _next();
      return KeyEventResult.handled;
    }
    if (key == LogicalKeyboardKey.arrowLeft) {
      _back();
      return KeyEventResult.handled;
    }
    return KeyEventResult.ignored;
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final draft = _draft;
    if (draft == null) {
      return ColoredBox(
        color: theme.colors.background,
        child: Center(
          child: MLoader(
            semanticsLabel: AppLocalizations.of(context)!.commonLoading,
          ),
        ),
      );
    }
    final duration = onboardingMotionDisabled(context)
        ? Duration.zero
        : const Duration(milliseconds: 260);

    return Focus(
      focusNode: _keyboardFocus,
      onKeyEvent: _onKey,
      child: Stack(
        fit: StackFit.expand,
        children: [
          OnboardingBackdrop(animate: !onboardingMotionDisabled(context)),
          Column(
            children: [
              _TopBar(
                appTitle: widget.appTitle,
                showSkip: !_last,
                busy: _busy,
                onSkip: _skip,
              ),
              Expanded(
                child: Row(
                  children: [
                    // Outside the switcher: the rail keeps its position and
                    // its own fill animation while pages change under it.
                    Padding(
                      padding: const EdgeInsetsDirectional.only(start: 26),
                      child: Center(
                        child: OnboardingProgressLane(
                          step: _index,
                          total: _steps.length,
                        ),
                      ),
                    ),
                    Expanded(
                      child: AnimatedSwitcher(
                        duration: duration,
                        switchInCurve: Curves.easeOutCubic,
                        switchOutCurve: Curves.easeInCubic,
                        transitionBuilder: (child, animation) => FadeTransition(
                          opacity: animation,
                          child: SlideTransition(
                            position: Tween<Offset>(
                              begin: const Offset(0, 0.015),
                              end: Offset.zero,
                            ).animate(animation),
                            child: child,
                          ),
                        ),
                        layoutBuilder: (current, previous) => Stack(
                          alignment: Alignment.center,
                          fit: StackFit.expand,
                          children: [...previous, ?current],
                        ),
                        child: KeyedSubtree(
                          key: ValueKey(_index),
                          child: _step(draft),
                        ),
                      ),
                    ),
                  ],
                ),
              ),
              _Footer(
                index: _index,
                last: _last,
                busy: _busy,
                onBack: _back,
                onNext: _next,
              ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _step(AppSettings draft) => switch (_steps[_index]) {
    OnboardingStep.welcome => OnboardingWelcomeStep(appTitle: widget.appTitle),
    OnboardingStep.language => OnboardingLanguageStep(
      draft: draft,
      onChanged: _apply,
    ),
    OnboardingStep.appearance => OnboardingAppearanceStep(
      draft: draft,
      onChanged: _apply,
    ),
    OnboardingStep.controllers => OnboardingControllersStep(
      draft: draft,
      controllers: _controllers,
      onChanged: _apply,
    ),
    OnboardingStep.library => const OnboardingLibraryStep(),
    OnboardingStep.privacy => OnboardingPrivacyStep(
      draft: draft,
      onChanged: _apply,
    ),
    OnboardingStep.ready => const OnboardingReadyStep(),
  };
}

/// Brand, step ticker, skip, window controls.
class _TopBar extends StatelessWidget {
  const new({
    required this.appTitle,
    required this.showSkip,
    required this.busy,
    required this.onSkip,
  });

  final String appTitle;
  final bool showSkip;
  final bool busy;
  final VoidCallback onSkip;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final l10n = AppLocalizations.of(context)!;
    final desktop = isDesktopWindow;
    Widget maybeDrag(Widget child) =>
        desktop ? DragToMoveArea(child: child) : child;

    return SizedBox(
      height: 64,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 8),
        child: Row(
          children: [
            maybeDrag(
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 16),
                child: Center(
                  child: Semantics(
                    label: appTitle,
                    image: true,
                    child: ColorFiltered(
                      colorFilter: ColorFilter.mode(
                        theme.colors.foreground,
                        BlendMode.srcIn,
                      ),
                      child: Image.asset(kMixarLogoAsset, height: 14),
                    ),
                  ),
                ),
              ),
            ),
            Expanded(child: maybeDrag(const SizedBox.expand())),
            if (!busy && showSkip)
              _SkipAction(
                key: kOnboardingSkipKey,
                label: l10n.onboardingSkip,
                onPress: onSkip,
              )
            else
              const SizedBox(height: 28, width: 84),
            if (desktop) const WindowTitleBarControls(),
          ],
        ),
      ),
    );
  }
}

/// Back and the primary action, with no divider of their own: the actions sit
/// on the stage rather than in a bar under it.
class _Footer extends StatelessWidget {
  const new({
    required this.index,
    required this.last,
    required this.busy,
    required this.onBack,
    required this.onNext,
  });

  final int index;
  final bool last;
  final bool busy;
  final VoidCallback onBack;
  final VoidCallback onNext;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return SizedBox(
      height: kOnboardingFooterHeight,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 24),
        child: Row(
          children: [
            if (index > 0)
              AppButton(
                key: kOnboardingBackKey,
                variant: MixarButtonVariant.ghost,
                size: MixarButtonSize.sm,
                mainAxisSize: MainAxisSize.min,
                onPress: busy ? null : onBack,
                child: _FooterActionLabel(
                  label: l10n.onboardingBack,
                  icon: LucideIcons.arrowLeft,
                ),
              ),
            const Spacer(),
            AppButton(
              key: kOnboardingNextKey,
              mainAxisSize: MainAxisSize.min,
              onPress: busy ? null : onNext,
              child: _FooterActionLabel(
                label: last ? l10n.onboardingStart : l10n.onboardingNext,
                icon: LucideIcons.arrowRight,
                iconTrailing: true,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// Top-right escape hatch: quiet text, never competing with the primary action.
class _SkipAction extends StatelessWidget {
  const new({required this.label, required this.onPress, super.key});

  final String label;
  final VoidCallback onPress;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return MTappable(
      onPress: onPress,
      semanticsLabel: label,
      builder: (_, state) {
        final ink = state.hovered || state.focused
            ? theme.colors.foreground
            : theme.colors.mutedForeground;
        return Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
          child: DecoratedBox(
            decoration: BoxDecoration(
              border: Border(
                bottom: BorderSide(
                  color: state.hovered || state.focused
                      ? theme.colors.accent
                      : const Color(0x00000000),
                ),
              ),
            ),
            child: Text(
              label.toUpperCase(),
              style: theme.typography.mono.xs.copyWith(
                color: ink,
                letterSpacing: 1.6,
                height: 1.2,
              ),
            ),
          ),
        );
      },
    );
  }
}

/// Icon + label row shared by both footer actions, so their sizing and
/// ellipsis behaviour cannot drift apart.
class _FooterActionLabel extends StatelessWidget {
  const new({
    required this.label,
    required this.icon,
    this.iconTrailing = false,
  });

  final String label;
  final IconData icon;
  final bool iconTrailing;

  @override
  Widget build(BuildContext context) {
    final glyph = Icon(icon, size: 15);
    final text = Flexible(
      child: Text(label, maxLines: 1, overflow: TextOverflow.ellipsis),
    );
    return Row(
      mainAxisSize: MainAxisSize.min,
      spacing: 8,
      children: iconTrailing ? [text, glyph] : [glyph, text],
    );
  }
}
