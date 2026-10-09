import 'package:flutter/widgets.dart';
import 'package:gui_flutter/mixer/pad_modes.dart';
import 'package:gui_flutter/mixer/pads/beat_jump_pads.dart';
import 'package:gui_flutter/mixer/pads/hot_cue_pads.dart';
import 'package:gui_flutter/mixer/pads/key_shift_pads.dart';
import 'package:gui_flutter/mixer/pads/keyboard_pads.dart';
import 'package:gui_flutter/mixer/pads/loop_roll_pads.dart';
import 'package:gui_flutter/mixer/pads/sampler_pads.dart';
import 'package:gui_flutter/mixer/pads/stems_pads.dart';
import 'package:gui_flutter/mixer/track_drag.dart';
import 'package:gui_flutter/shell/mixar_select.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';

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
    required this.onKeyShiftPress,
    required this.onKeyboardPress,
    required this.onKeyboardRelease,
    this.activeBankId,
    this.onSamplerAssign,
    this.stemMute = const [false, false, false, false],
    this.stemIsolate,
    this.stemsReady = false,
    this.stemsGenerating = false,
    this.onStemsPress,
    this.keyShiftSemitones = 0,
    this.keyboardPage = kDefaultPitchPage,
    this.keyShiftPage = kDefaultPitchPage,
    this.keyboardRootHotCue = 0,
    this.onSelectRoot,
    this.onPrevPage,
    this.onNextPage,
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
  final void Function(int slot, TrackDragPayload payload)? onSamplerAssign;
  final List<bool> stemMute;
  final int? stemIsolate;
  final bool stemsReady;
  final bool stemsGenerating;
  final ValueChanged<int>? onStemsPress;
  final ValueChanged<int> onKeyShiftPress;
  final ValueChanged<int> onKeyboardPress;
  final ValueChanged<int> onKeyboardRelease;
  final int keyShiftSemitones;
  final int keyboardPage;
  final int keyShiftPage;
  final int keyboardRootHotCue;
  final ValueChanged<int>? onSelectRoot;

  /// Steps the Keyboard / Key Shift semitone page backward / forward.
  final VoidCallback? onPrevPage;
  final VoidCallback? onNextPage;

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
    final panelSurface = Color.alphaBlend(
      theme.colors.background.withValues(alpha: 0.8),
      theme.colors.card,
    );
    final panelBorderColor = Color.alphaBlend(
      theme.colors.border,
      panelSurface,
    );

    final body = Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        DecoratedBox(
          decoration: BoxDecoration(
            border: Border(bottom: BorderSide(color: theme.colors.border)),
          ),
          child: MixarSelect<PadMode>(
            value: padMode,
            options: kPadModes,
            labelBuilder: padModeShortLabel,
            unfocusAfterPointerSelection: true,
            borderRadius: BorderRadius.only(
              topRight: theme.style.borderRadius.sm.topRight,
            ),
            borderColor: panelBorderColor,
            // The card frame, header divider and rail already draw these
            // edges; the selector only strokes the ones it owns.
            borderSides: const {
              MixarBorderSide.top,
              MixarBorderSide.right,
              MixarBorderSide.bottom,
            },
            onChanged: onPadMode,
            enabled: !disabled,
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
        banks: samplerBanks,
        activeBankId: activeBankId,
        disabled: _controlsDisabled,
        holdLike: _holdLike,
        effectivePlayMode: _effectivePlayMode,
        onPress: onSamplerPress,
        onRelease: onSamplerRelease,
        onSelectBank: onSelectBank,
        onSaveBank: onSaveBank,
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
        rootHotCue: keyboardRootHotCue,
        hotCues: hotCues,
        onSelectRoot: onSelectRoot ?? (_) {},
        disabled: _controlsDisabled,
        onPress: onKeyboardPress,
        onRelease: onKeyboardRelease,
        onPrevPage: onPrevPage ?? () {},
        onNextPage: onNextPage ?? () {},
      ),
      PadMode.keyShift => KeyShiftPads(
        page: keyShiftPage,
        activeSemitones: keyShiftSemitones,
        disabled: _controlsDisabled,
        onPress: onKeyShiftPress,
        onPrevPage: onPrevPage ?? () {},
        onNextPage: onNextPage ?? () {},
      ),
    };
  }
}
