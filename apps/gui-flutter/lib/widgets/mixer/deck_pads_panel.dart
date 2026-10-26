import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/models/pad_modes.dart';
import 'package:gui_flutter/core/theme/mixar_button_style.dart';
import 'package:gui_flutter/core/utils/track_drag.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/src/rust/api/engine.dart';
import 'package:gui_flutter/widgets/mixer/pads/beat_jump_pads.dart';
import 'package:gui_flutter/widgets/mixer/pads/hot_cue_pads.dart';
import 'package:gui_flutter/widgets/mixer/pads/key_shift_pads.dart';
import 'package:gui_flutter/widgets/mixer/pads/keyboard_pads.dart';
import 'package:gui_flutter/widgets/mixer/pads/loop_roll_pads.dart';
import 'package:gui_flutter/widgets/mixer/pads/sampler_pads.dart';
import 'package:gui_flutter/providers/shell/mixar_overlay_controller.dart';
import 'package:gui_flutter/widgets/mixer/mixer_button.dart';
import 'package:gui_flutter/widgets/mixer/pads/stems_pads.dart';
import 'package:gui_flutter/widgets/shell/mixar_dialog.dart';
import 'package:gui_flutter/widgets/shell/mixar_input.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_anchor.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_body.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_group.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_item.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_label.dart';
import 'package:gui_flutter/widgets/shell/mixar_select.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Width of the gear cell in the pads header.
///
/// The cell spans the header's full height and sits flush against the panel
/// edge, so the button owns the whole corner instead of floating inside it.
const _kConfigCellWidth = 36.0;

/// Presentational deck pads panel (mode select + per-mode grids).
class DeckPadsPanel extends StatelessWidget {
  const new({
    required this.padMode,
    required this.onPadMode,
    required this.hotCues,
    required this.onHotCuePress,
    required this.onHotCueRelease,
    required this.onLoopRollPress,
    required this.onLoopRollRelease,
    required this.onBeatJumpPress,
    required this.onBeatJumpRelease,
    required this.samplerSlots,
    required this.samplerBanks,
    required this.onSamplerPress,
    required this.onSamplerRelease,
    required this.onSelectBank,
    required this.onSaveBank,
    required this.onCreateBank,
    required this.onSelectPage,
    required this.onKeyShiftPress,
    required this.onKeyboardPress,
    required this.onKeyboardRelease,
    this.activeBankId,
    this.onSamplerAssign,
    this.stemMute = const [false, false, false, false],
    this.stemIsolate,
    this.stemsReady = false,
    this.stemsLive = false,
    this.stemsGenerating = false,
    this.realtimeStemsEnabled = false,
    this.onStemsPress,
    this.keyShiftSemitones = 0,
    this.keyboardPage = kDefaultPitchPage,
    this.keyShiftPage = kDefaultPitchPage,
    this.keyboardRootHotCue = 0,
    this.onSelectRoot,
    this.hasTrack = false,
    this.disabled = false,
    this.bordered = true,
    super.key,
  });

  final PadMode padMode;
  final ValueChanged<PadMode> onPadMode;
  final List<DeckHotCue> hotCues;
  final void Function(int slot, bool shift) onHotCuePress;
  final ValueChanged<int> onHotCueRelease;
  final ValueChanged<int> onLoopRollPress;
  final ValueChanged<int> onLoopRollRelease;
  final ValueChanged<int> onBeatJumpPress;
  final ValueChanged<int> onBeatJumpRelease;
  final List<SamplerSlot> samplerSlots;
  final List<SamplerBank> samplerBanks;
  final String? activeBankId;
  final void Function(int slot, bool shift) onSamplerPress;
  final ValueChanged<int> onSamplerRelease;
  final ValueChanged<String> onSelectBank;
  final void Function(String bankId, String name, String? playMode) onSaveBank;
  final void Function(String name, String? playMode) onCreateBank;
  final void Function(int slot, TrackDragPayload payload)? onSamplerAssign;
  final List<bool> stemMute;
  final int? stemIsolate;
  final bool stemsReady;
  final bool stemsLive;
  final bool stemsGenerating;
  final bool realtimeStemsEnabled;
  final ValueChanged<int>? onStemsPress;
  final ValueChanged<int> onKeyShiftPress;
  final ValueChanged<int> onKeyboardPress;
  final ValueChanged<int> onKeyboardRelease;
  final int keyShiftSemitones;
  final int keyboardPage;
  final int keyShiftPage;
  final int keyboardRootHotCue;
  final ValueChanged<int>? onSelectRoot;

  /// Jumps the active Keyboard / Key Shift mode to [page].
  final ValueChanged<int> onSelectPage;

  final bool hasTrack;
  final bool disabled;
  final bool bordered;

  bool get _controlsDisabled => disabled || !hasTrack;

  String get _effectivePlayMode {
    for (final bank in samplerBanks) {
      if (bank.id == activeBankId) {
        return bank.playMode ?? kDefaultSamplerPlayMode;
      }
    }
    return kDefaultSamplerPlayMode;
  }

  bool get _holdLike {
    final mode = _effectivePlayMode;
    return mode == kSamplerPlayModeHold || mode == kSamplerPlayModeLoop;
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final l10n = AppLocalizations.of(context);
    // Keeps the config cell inside the card frame so its wash never covers the
    // frame's strokes. Unbordered panels sit in the tabs container, which paints
    // its frame above the content, so they need no inset.
    final frameInset = bordered ? theme.style.borderWidth : 0.0;

    final body = Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        DecoratedBox(
          decoration: BoxDecoration(
            border: Border(bottom: BorderSide(color: theme.colors.border)),
          ),
          // The config button shares the header row so it never takes height
          // from the pads: every mode's grid sees the same box. It lives in a
          // positioned cell that spans the header (top to divider, flush to the
          // panel edge) so the whole corner is the button, not a floating icon.
          child: Stack(
            children: [
              Row(
                children: [
                  Expanded(
                    child: MixarSelect<PadMode>(
                      dialogTitle: 'Pad mode',
                      value: padMode,
                      options: kPadModes,
                      labelBuilder: (mode) => _modeLabel(mode, l10n),
                      optionLabelBuilder: padModeShortLabel,
                      labelSpanBuilder: (mode) => _modeLabelSpan(
                        mode,
                        theme.colors.mutedForeground,
                        l10n,
                      ),
                      unfocusAfterPointerSelection: true,
                      // Only shapes the focus ring: the header divider and the
                      // card frame draw every stroke.
                      borderRadius: _showsConfig
                          ? null
                          : BorderRadius.only(
                              topRight: theme.style.borderRadius.sm.topRight,
                            ),
                      borderColor: theme.colors.border,
                      // The header divider and the card frame draw every edge
                      // from the same border token. A stroke of its own would
                      // paint a second, differently blended shade over the
                      // divider.
                      borderSides: const <MixarBorderSide>{},
                      onChanged: onPadMode,
                      enabled: !disabled,
                    ),
                  ),
                  // Reserve the gear's cell so the selector's chevron stays out
                  // of its hit area.
                  if (_showsConfig) const SizedBox(width: _kConfigCellWidth),
                ],
              ),
              if (_showsConfig)
                Positioned(
                  // The select ends where the reserved cell begins; the insets
                  // only trim the cell back inside the header, so the two never
                  // overlap. The bottom always clears the divider, which the
                  // panel paints itself in both framed and tabs-hosted layouts;
                  // the top and right only need clearing inside a card frame.
                  top: frameInset,
                  bottom: theme.style.borderWidth,
                  right: frameInset,
                  width: _kConfigCellWidth - frameInset,
                  child: _configButton(context),
                ),
            ],
          ),
        ),
        Expanded(child: _modeBody()),
      ],
    );

    if (!bordered) {
      return body;
    }

    return DecoratedBox(
      decoration: BoxDecoration(
        border: Border.all(color: theme.colors.border),
        borderRadius: theme.style.borderRadius.sm,
        color: theme.colors.background.withValues(alpha: 0.8),
      ),
      child: body,
    );
  }

  Widget _modeBody() {
    return switch (padMode) {
      PadMode.hotCue => HotCuePads(
        hotCues: hotCues,
        disabled: _controlsDisabled,
        onPress: onHotCuePress,
        onRelease: onHotCueRelease,
      ),
      PadMode.loopRoll => LoopRollPads(
        disabled: _controlsDisabled,
        onPress: onLoopRollPress,
        onRelease: onLoopRollRelease,
      ),
      PadMode.beatJump => BeatJumpPads(
        disabled: _controlsDisabled,
        onPress: onBeatJumpPress,
        onRelease: onBeatJumpRelease,
      ),
      PadMode.sampler => SamplerPads(
        slots: samplerSlots,
        disabled: _controlsDisabled,
        holdLike: _holdLike,
        onPress: onSamplerPress,
        onRelease: onSamplerRelease,
        onAssign: onSamplerAssign,
      ),
      PadMode.stems => StemsPads(
        stemMute: stemMute,
        stemIsolate: stemIsolate,
        stemsReady: stemsReady,
        stemsGenerating: stemsGenerating,
        disabled: _controlsDisabled,
        onPress: onStemsPress ?? (_) {},
      ),
      PadMode.keyboard => KeyboardPads(
        page: keyboardPage,
        disabled: _controlsDisabled,
        onPress: onKeyboardPress,
        onRelease: onKeyboardRelease,
      ),
      PadMode.keyShift => KeyShiftPads(
        page: keyShiftPage,
        activeSemitones: keyShiftSemitones,
        disabled: _controlsDisabled,
        onPress: onKeyShiftPress,
      ),
    };
  }

  SamplerBank? get _resolvedBank {
    for (final bank in samplerBanks) {
      if (bank.id == activeBankId) {
        return bank;
      }
    }
    return samplerBanks.isNotEmpty ? samplerBanks.first : null;
  }

  bool get _showsConfig =>
      padMode == PadMode.sampler ||
      padMode == PadMode.keyboard ||
      padMode == PadMode.keyShift;

  /// Mode label parts: the selected mode can carry a bank / page / source
  /// suffix. The suffix is rendered muted; [labelBuilder] keeps the plain
  /// string for semantics and fallback.
  ({String label, String? suffix}) _modeParts(
    PadMode mode,
    AppLocalizations? l10n,
  ) {
    final label = padModeShortLabel(mode);
    if (mode != padMode) {
      return (label: label, suffix: null);
    }
    return switch (mode) {
      PadMode.hotCue ||
      PadMode.loopRoll ||
      PadMode.beatJump => (label: label, suffix: null),
      PadMode.stems =>
        realtimeStemsEnabled && (stemsReady || stemsLive)
            ? (
                label: label,
                suffix: stemsLive
                    ? l10n?.padStemsRealtime ?? 'Realtime'
                    : l10n?.padStemsOffline ?? 'Offline',
              )
            : (label: label, suffix: null),
      PadMode.sampler => switch (_resolvedBank) {
        final bank? => (label: label, suffix: bank.name),
        null => (label: label, suffix: null),
      },
      PadMode.keyboard => (
        label: label,
        suffix: keyboardPageRangeLabel(keyboardPage),
      ),
      PadMode.keyShift => (
        label: label,
        suffix: keyShiftPageRangeLabel(keyShiftPage),
      ),
    };
  }

  String _modeLabel(PadMode mode, AppLocalizations? l10n) {
    final parts = _modeParts(mode, l10n);
    return parts.suffix == null
        ? parts.label
        : '${parts.label} (${parts.suffix})';
  }

  InlineSpan _modeLabelSpan(PadMode mode, Color muted, AppLocalizations? l10n) {
    final parts = _modeParts(mode, l10n);
    return TextSpan(
      children: [
        TextSpan(text: parts.label),
        if (parts.suffix != null)
          TextSpan(
            text: ' (${parts.suffix})',
            style: TextStyle(color: muted),
          ),
      ],
    );
  }

  Widget _configButton(BuildContext context) {
    return MixarMenuAnchor(
      dialogTitle: '${padModeShortLabel(padMode)} settings',
      minWidth: 220,
      enabled: !disabled,
      menuBuilder: (menuContext, controller) =>
          _configMenu(context, menuContext, controller),
      childBuilder: (context, controller) => SizedBox.expand(
        child: ListenableBuilder(
          listenable: controller,
          builder: (context, _) {
            final showing = controller.isShowing;
            final theme = context.theme;
            return MixerButton.icon(
              semanticsLabel: '${padModeShortLabel(padMode)} settings',
              // Ghost: the header already draws the chrome, so the gear is a
              // bare icon while the whole cell stays the hit and hover surface.
              variant: MixarButtonVariant.ghost,
              size: MixarButtonSize.xs,
              selected: showing,
              onPress: controller.toggle,
              child: Icon(
                LucideIcons.settings2,
                size: 16,
                color: showing ? theme.colors.selectionForeground : null,
              ),
            );
          },
        ),
      ),
    );
  }

  Widget _configMenu(
    BuildContext hostContext,
    BuildContext menuContext,
    MixarOverlayController controller,
  ) {
    final theme = menuContext.theme;
    final groups = <Widget>[];

    if (padMode == PadMode.sampler) {
      final active = _resolvedBank;
      groups.add(
        MixarMenuGroup(
          children: [
            const MixarMenuLabel(child: Text('Bank')),
            if (samplerBanks.isEmpty)
              const MixarMenuItem(title: Text('No banks'), enabled: false)
            else
              ConstrainedBox(
                constraints: const BoxConstraints(maxHeight: 240),
                child: SingleChildScrollView(
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      for (final bank in samplerBanks)
                        MixarMenuItem(
                          title: Row(
                            children: [
                              Expanded(child: Text(bank.name)),
                              ?_selectionCheck(
                                theme,
                                selected: bank.id == active?.id,
                              ),
                            ],
                          ),
                          onPress: () {
                            controller.hide();
                            onSelectBank(bank.id);
                          },
                        ),
                    ],
                  ),
                ),
              ),
          ],
        ),
      );
      groups.add(
        MixarMenuGroup(
          children: [
            MixarMenuItem(
              title: const Text('New bank…'),
              onPress: () {
                controller.hide();
                if (hostContext.mounted) {
                  unawaited(_showBankDialog(hostContext, null));
                }
              },
            ),
            if (active != null)
              MixarMenuItem(
                title: const Text('Edit bank…'),
                onPress: () {
                  controller.hide();
                  if (hostContext.mounted) {
                    unawaited(_showBankDialog(hostContext, active));
                  }
                },
              ),
          ],
        ),
      );
    } else {
      if (padMode == PadMode.keyboard) {
        final cues = [...hotCues]..sort((a, b) => a.slot.compareTo(b.slot));
        groups.add(
          MixarMenuGroup(
            children: [
              const MixarMenuLabel(child: Text('Root hot cue')),
              if (cues.isEmpty)
                const MixarMenuItem(title: Text('No hot cues'), enabled: false)
              else
                for (final cue in cues)
                  MixarMenuItem(
                    title: Row(
                      children: [
                        Expanded(child: Text(_cueLabel(cue))),
                        ?_selectionCheck(
                          theme,
                          selected: cue.slot == keyboardRootHotCue,
                        ),
                      ],
                    ),
                    onPress: () {
                      controller.hide();
                      onSelectRoot?.call(cue.slot);
                    },
                  ),
            ],
          ),
        );
      }

      final isKeyboard = padMode == PadMode.keyboard;
      final count = isKeyboard ? kKeyboardPageCount : kKeyShiftPageCount;
      final current = isKeyboard ? keyboardPage : keyShiftPage;
      String pageLabel(int page) => isKeyboard
          ? keyboardPageRangeLabel(page)
          : keyShiftPageRangeLabel(page);
      groups.add(
        MixarMenuGroup(
          children: [
            const MixarMenuLabel(child: Text('Page')),
            for (var page = 1; page <= count; page++)
              MixarMenuItem(
                title: Row(
                  children: [
                    Expanded(child: Text(pageLabel(page))),
                    ?_selectionCheck(theme, selected: page == current),
                  ],
                ),
                onPress: () {
                  controller.hide();
                  onSelectPage(page);
                },
              ),
          ],
        ),
      );
    }

    return MixarMenuBody(groups: groups);
  }

  /// Selection check used by every config-menu list.
  Widget? _selectionCheck(MixarThemeData theme, {required bool selected}) {
    return selected
        ? Icon(LucideIcons.check, size: 14, color: theme.colors.foreground)
        : null;
  }

  String _cueLabel(DeckHotCue cue) {
    final name = cue.label?.trim();
    return name != null && name.isNotEmpty ? name : 'Hot cue ${cue.slot + 1}';
  }

  Future<void> _showBankDialog(BuildContext context, SamplerBank? bank) async {
    var name = bank?.name ?? 'Bank ${samplerBanks.length + 1}';
    var modeValue = bank?.playMode ?? 'default';
    final result = await showMixarDialog<(String, String?)?>(
      context: context,
      builder: (context) {
        return StatefulBuilder(
          builder: (context, setLocal) {
            final theme = context.theme;
            return Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                mainAxisSize: .min,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Text(
                    bank == null ? 'New bank' : 'Bank settings',
                    style: theme.typography.body.md.copyWith(
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                  const SizedBox(height: 12),
                  MixarInput(
                    label: const Text('Name'),
                    initialValue: name,
                    onChanged: (v) {
                      name = v;
                      setLocal(() {});
                    },
                  ),
                  const SizedBox(height: 12),
                  Text('Play mode', style: theme.typography.body.sm),
                  const SizedBox(height: 6),
                  Wrap(
                    spacing: 6,
                    runSpacing: 6,
                    children: [
                      for (final opt in kSamplerPlayModeOptions)
                        MixerButton(
                          variant: modeValue == opt
                              ? MixarButtonVariant.primary
                              : MixarButtonVariant.secondary,
                          onPress: () => setLocal(() => modeValue = opt),
                          child: Text(opt),
                        ),
                    ],
                  ),
                  const SizedBox(height: 16),
                  Row(
                    children: [
                      Expanded(
                        child: MixerButton(
                          variant: MixarButtonVariant.secondary,
                          onPress: () => Navigator.of(context).pop(),
                          child: const Text('Cancel'),
                        ),
                      ),
                      const SizedBox(width: 8),
                      Expanded(
                        child: MixerButton(
                          onPress: name.trim().isEmpty
                              ? null
                              : () {
                                  final playMode = modeValue == 'default'
                                      ? null
                                      : modeValue;
                                  Navigator.of(context).pop((name, playMode));
                                },
                          child: Text(bank == null ? 'Create' : 'Save'),
                        ),
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
    if (result == null) {
      return;
    }
    final trimmed = result.$1.trim();
    if (trimmed.isEmpty) {
      return;
    }
    if (bank == null) {
      onCreateBank(trimmed, result.$2);
    } else {
      onSaveBank(bank.id, trimmed, result.$2);
    }
  }
}
