import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/pad_modes.dart';
import 'package:gui_flutter/mixer/pads/beat_jump_pads.dart';
import 'package:gui_flutter/mixer/pads/hot_cue_pads.dart';
import 'package:gui_flutter/mixer/pads/loop_roll_pads.dart';
import 'package:gui_flutter/mixer/pads/sampler_pads.dart';
import 'package:gui_flutter/mixer/track_drag.dart';

/// Presentational deck pads panel (mode tabs + per-mode grids).
class DeckPadsPanel extends StatelessWidget {
  const DeckPadsPanel({
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
    this.activeBankId,
    this.onSamplerAssign,
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

    final body = Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        DecoratedBox(
          decoration: BoxDecoration(
            border: Border(bottom: BorderSide(color: theme.colors.border)),
          ),
          child: Row(
            children: [
              for (final mode in kPadModes)
                Expanded(
                  child: _PadModeTab(
                    label: padModeShortLabel(mode),
                    active: padMode == mode,
                    disabled: disabled,
                    onPress: () => onPadMode(mode),
                  ),
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
        borderRadius: BorderRadius.circular(8),
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
    };
  }
}

class _PadModeTab extends StatelessWidget {
  const _PadModeTab({
    required this.label,
    required this.active,
    required this.onPress,
    this.disabled = false,
  });

  final String label;
  final bool active;
  final VoidCallback onPress;
  final bool disabled;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final fg = disabled
        ? theme.colors.mutedForeground.withValues(alpha: 0.45)
        : active
        ? theme.colors.foreground
        : theme.colors.mutedForeground;

    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: disabled ? null : onPress,
      child: ColoredBox(
        color: active
            ? theme.colors.secondary.withValues(alpha: 0.55)
            : const Color(0x00000000),
        child: Padding(
          padding: const EdgeInsets.symmetric(vertical: 6, horizontal: 2),
          child: Text(
            label.toUpperCase(),
            textAlign: TextAlign.center,
            style: theme.typography.body.xs.copyWith(
              fontWeight: FontWeight.w700,
              letterSpacing: 1.2,
              color: fg,
            ),
          ),
        ),
      ),
    );
  }
}
