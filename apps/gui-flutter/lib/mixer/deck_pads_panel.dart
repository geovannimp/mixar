import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/pad_modes.dart';
import 'package:gui_flutter/mixer/pads/beat_jump_pads.dart';
import 'package:gui_flutter/mixer/pads/hot_cue_pads.dart';
import 'package:gui_flutter/mixer/pads/loop_roll_pads.dart';
import 'package:gui_flutter/mixer/pads/sampler_pads.dart';

/// Tauri-shaped deck pads panel (mode tabs + per-mode grids). Local state only.
class DeckPadsPanel extends StatefulWidget {
  const DeckPadsPanel({this.hasTrack = false, this.disabled = false, super.key});

  /// When false, pad actions are disabled (Tauri `!deck.track`).
  final bool hasTrack;

  /// Disables mode tabs (and pads).
  final bool disabled;

  @override
  State<DeckPadsPanel> createState() => _DeckPadsPanelState();
}

class _DeckPadsPanelState extends State<DeckPadsPanel> {
  PadMode _padMode = PadMode.hotCue;
  final List<DeckHotCue> _hotCues = [
    // Demo filled cue so accent chrome is visible while unloaded.
    const DeckHotCue(slot: 0, positionMs: 12500),
  ];

  late List<SamplerBank> _banks;
  late String _activeBankId;
  late List<SamplerSlot> _slots;
  final String _settingsPlayMode = kDefaultSamplerPlayMode;

  @override
  void initState() {
    super.initState();
    _banks = const [
      SamplerBank(id: 'bank-1', name: 'Bank 1'),
      SamplerBank(id: 'bank-2', name: 'Bank 2', playMode: kSamplerPlayModeHold),
    ];
    _activeBankId = _banks.first.id;
    _slots = [
      const SamplerSlot(label: 'Kick', durationMs: 500, path: 'demo'),
      for (var i = 1; i < 8; i++) const SamplerSlot(),
    ];
  }

  bool get _controlsDisabled => widget.disabled || !widget.hasTrack;

  String get _effectivePlayMode {
    for (final bank in _banks) {
      if (bank.id == _activeBankId) {
        return bank.playMode ?? _settingsPlayMode;
      }
    }
    return _settingsPlayMode;
  }

  bool get _holdLike {
    final mode = _effectivePlayMode;
    return mode == kSamplerPlayModeHold || mode == kSamplerPlayModeLoop;
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;

    return DecoratedBox(
      decoration: BoxDecoration(
        border: Border.all(color: theme.colors.border),
        borderRadius: BorderRadius.circular(8),
        color: theme.colors.background.withValues(alpha: 0.8),
      ),
      child: Column(
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
                      active: _padMode == mode,
                      disabled: widget.disabled,
                      onPress: () => setState(() => _padMode = mode),
                    ),
                  ),
              ],
            ),
          ),
          Expanded(child: _modeBody()),
        ],
      ),
    );
  }

  Widget _modeBody() {
    return switch (_padMode) {
      PadMode.hotCue => HotCuePads(
        hotCues: _hotCues,
        disabled: _controlsDisabled,
        onTrigger: (_) {},
        onSave: (slot) {
          setState(() {
            _hotCues.removeWhere((c) => c.slot == slot);
            _hotCues.add(DeckHotCue(slot: slot, positionMs: slot * 1000));
          });
        },
        onDelete: (slot) {
          setState(() => _hotCues.removeWhere((c) => c.slot == slot));
        },
      ),
      PadMode.loopRoll => LoopRollPads(
        disabled: _controlsDisabled,
        onBegin: (_) {},
        onEnd: () {},
      ),
      PadMode.beatJump => BeatJumpPads(
        disabled: _controlsDisabled,
        onBeatJump: (_) {},
      ),
      PadMode.sampler => SamplerPads(
        slots: _slots,
        banks: _banks,
        activeBankId: _activeBankId,
        disabled: _controlsDisabled,
        holdLike: _holdLike,
        effectivePlayMode: _effectivePlayMode,
        onTrigger: (_) {},
        onEnd: (_) {},
        onClear: (slot) {
          setState(() {
            _slots = [
              for (var i = 0; i < _slots.length; i++)
                if (i == slot) const SamplerSlot() else _slots[i],
            ];
          });
        },
        onSelectBank: (id) => setState(() => _activeBankId = id),
        onSaveBank: (bankId, name, playMode) {
          setState(() {
            _banks = [
              for (final bank in _banks)
                if (bank.id == bankId)
                  SamplerBank(id: bank.id, name: name, playMode: playMode)
                else
                  bank,
            ];
          });
        },
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
