import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/pad_format.dart';
import 'package:gui_flutter/mixer/pad_modes.dart';
import 'package:gui_flutter/mixer/pads/pad_button.dart';
import 'package:gui_flutter/mixer/pads/pad_grid.dart';

class SamplerSlot {
  const SamplerSlot({this.label, this.durationMs, this.path});

  final String? label;
  final int? durationMs;
  final String? path;

  bool get filled =>
      (path != null && path!.isNotEmpty) ||
      (label != null && label!.trim().isNotEmpty);
}

class SamplerBank {
  const SamplerBank({
    required this.id,
    required this.name,
    this.playMode,
  });

  final String id;
  final String name;

  /// Null = inherit default (`oneshot`).
  final String? playMode;

  SamplerBank copyWith({String? name, String? playMode, bool clearPlayMode = false}) =>
      SamplerBank(
        id: id,
        name: name ?? this.name,
        playMode: clearPlayMode ? null : (playMode ?? this.playMode),
      );
}

class SamplerPads extends StatelessWidget {
  const SamplerPads({
    required this.slots,
    required this.banks,
    required this.activeBankId,
    required this.holdLike,
    required this.effectivePlayMode,
    required this.onTrigger,
    required this.onEnd,
    required this.onClear,
    required this.onSelectBank,
    required this.onSaveBank,
    this.disabled = false,
    super.key,
  });

  final List<SamplerSlot> slots;
  final List<SamplerBank> banks;
  final String? activeBankId;
  final bool holdLike;
  final String effectivePlayMode;
  final ValueChanged<int> onTrigger;
  final ValueChanged<int> onEnd;
  final ValueChanged<int> onClear;
  final ValueChanged<String> onSelectBank;
  final void Function(String bankId, String name, String? playMode) onSaveBank;
  final bool disabled;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final activeBankIndex = banks.indexWhere((b) => b.id == activeBankId);
    final activeBank = activeBankIndex >= 0 ? banks[activeBankIndex] : null;

    void cycleBank(int direction) {
      if (banks.isEmpty) {
        return;
      }
      final current = activeBankIndex >= 0 ? activeBankIndex : 0;
      final next = (current + direction + banks.length * 8) % banks.length;
      onSelectBank(banks[next].id);
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        DecoratedBox(
          decoration: BoxDecoration(
            border: Border(
              bottom: BorderSide(color: theme.colors.border),
            ),
          ),
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 4),
            child: Row(
              children: [
                _BankChromeButton(
                  label: '◀',
                  disabled: disabled || banks.length < 2,
                  onPress: () => cycleBank(-1),
                ),
                Expanded(
                  child: Row(
                    children: [
                      const Spacer(),
                      Flexible(
                        child: Text(
                          activeBank?.name ?? 'No bank',
                          overflow: TextOverflow.ellipsis,
                          textAlign: TextAlign.center,
                          style: theme.typography.body.xs.copyWith(
                            fontWeight: FontWeight.w600,
                            fontFamily: 'monospace',
                          ),
                        ),
                      ),
                      Expanded(
                        child: Align(
                          alignment: Alignment.centerLeft,
                          child: effectivePlayMode != kDefaultSamplerPlayMode
                              ? Padding(
                                  padding: const EdgeInsets.only(left: 6),
                                  child: DecoratedBox(
                                    decoration: BoxDecoration(
                                      border: Border.all(
                                        color: theme.colors.border,
                                      ),
                                      borderRadius: BorderRadius.circular(4),
                                      color: theme.colors.secondary
                                          .withValues(alpha: 0.5),
                                    ),
                                    child: Padding(
                                      padding: const EdgeInsets.symmetric(
                                        horizontal: 4,
                                        vertical: 1,
                                      ),
                                      child: Text(
                                        effectivePlayMode,
                                        style: theme.typography.body.xs
                                            .copyWith(
                                              fontWeight: FontWeight.w700,
                                            ),
                                      ),
                                    ),
                                  ),
                                )
                              : const SizedBox.shrink(),
                        ),
                      ),
                    ],
                  ),
                ),
                _BankChromeButton(
                  label: '▶',
                  disabled: disabled || banks.length < 2,
                  onPress: () => cycleBank(1),
                ),
                _BankChromeButton(
                  label: '⚙',
                  disabled: disabled || activeBank == null,
                  onPress: activeBank == null
                      ? null
                      : () => _openBankConfig(context, activeBank),
                ),
              ],
            ),
          ),
        ),
        Expanded(
          child: PadGrid(
            children: [
              for (var slot = 0; slot < 8; slot++)
                () {
                  final sample = slot < slots.length
                      ? slots[slot]
                      : const SamplerSlot();
                  final filled = sample.filled;
                  final label = sample.label?.trim();
                  return PadButton(
                    disabled: disabled,
                    accentSlot: filled ? slot : null,
                    tooltip: filled
                        ? (holdLike
                              ? 'Pad ${slot + 1} — hold to play, shift+click clear'
                              : 'Pad ${slot + 1} — click trigger, shift+click clear')
                        : 'Sampler pad ${slot + 1} (assign when library DnD lands)',
                    onPress: () {
                      if (shiftKeyPressed() && filled) {
                        onClear(slot);
                        return;
                      }
                      if (filled && !holdLike) {
                        onTrigger(slot);
                      }
                    },
                    onPointerDown: filled && holdLike
                        ? () => onTrigger(slot)
                        : null,
                    onPointerUp: filled && holdLike
                        ? () => onEnd(slot)
                        : null,
                    onPointerCancel: filled && holdLike
                        ? () => onEnd(slot)
                        : null,
                    child: Column(
                      mainAxisSize: .min,
                      children: [
                        Text(
                          filled && label != null && label.isNotEmpty
                              ? label
                              : '${slot + 1}',
                          overflow: TextOverflow.ellipsis,
                          style: theme.typography.body.xs.copyWith(
                            fontWeight: FontWeight.w700,
                          ),
                        ),
                        Text(
                          filled && sample.durationMs != null
                              ? formatDeckTimeTenth(sample.durationMs)
                              : 'sample',
                          overflow: TextOverflow.ellipsis,
                          style: theme.typography.body.xs,
                        ),
                      ],
                    ),
                  );
                }(),
            ],
          ),
        ),
      ],
    );
  }

  Future<void> _openBankConfig(BuildContext context, SamplerBank bank) async {
    var name = bank.name;
    var modeValue = bank.playMode ?? 'default';
    final result = await showFDialog<(String, String?)?>(
      context: context,
      builder: (context, style, animation) {
        return FDialog(
          animation: animation,
          builder: (context, style) {
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
                        'Bank settings',
                        style: theme.typography.body.md.copyWith(
                          fontWeight: FontWeight.w700,
                        ),
                      ),
                      const SizedBox(height: 12),
                      FTextField(
                        label: const Text('Name'),
                        control: .managed(
                          initial: TextEditingValue(text: name),
                          onChange: (v) => name = v.text,
                        ),
                      ),
                      const SizedBox(height: 12),
                      Text(
                        'Play mode',
                        style: theme.typography.body.sm,
                      ),
                      const SizedBox(height: 6),
                      Wrap(
                        spacing: 6,
                        runSpacing: 6,
                        children: [
                          for (final opt in const [
                            'default',
                            'oneshot',
                            'hold',
                            'loop',
                          ])
                            FButton(
                              variant: modeValue == opt
                                  ? .primary
                                  : .secondary,
                              onPress: () => setLocal(() => modeValue = opt),
                              child: Text(opt),
                            ),
                        ],
                      ),
                      const SizedBox(height: 16),
                      Row(
                        children: [
                          Expanded(
                            child: FButton(
                              variant: .secondary,
                              onPress: () => Navigator.of(context).pop(),
                              child: const Text('Cancel'),
                            ),
                          ),
                          const SizedBox(width: 8),
                          Expanded(
                            child: FButton(
                              onPress: () {
                                final playMode = modeValue == 'default'
                                    ? null
                                    : modeValue;
                                Navigator.of(context).pop((name, playMode));
                              },
                              child: const Text('Save'),
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
      },
    );
    if (result == null) {
      return;
    }
    onSaveBank(bank.id, result.$1.trim().isEmpty ? bank.name : result.$1.trim(), result.$2);
  }
}

class _BankChromeButton extends StatelessWidget {
  const _BankChromeButton({
    required this.label,
    required this.onPress,
    this.disabled = false,
  });

  final String label;
  final VoidCallback? onPress;
  final bool disabled;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return GestureDetector(
      onTap: disabled ? null : onPress,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 4),
        child: Text(
          label,
          style: theme.typography.body.xs.copyWith(
            color: disabled
                ? theme.colors.mutedForeground.withValues(alpha: 0.4)
                : theme.colors.mutedForeground,
          ),
        ),
      ),
    );
  }
}
