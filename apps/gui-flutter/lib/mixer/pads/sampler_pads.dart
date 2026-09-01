import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/pad_format.dart';
import 'package:gui_flutter/mixer/pad_modes.dart';
import 'package:gui_flutter/mixer/pads/pad_button.dart';
import 'package:gui_flutter/shell/app_typography.dart';
import 'package:gui_flutter/mixer/pads/pad_grid.dart';
import 'package:gui_flutter/mixer/track_drag.dart';
import 'package:super_drag_and_drop/super_drag_and_drop.dart';

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
  const SamplerBank({required this.id, required this.name, this.playMode});

  final String id;
  final String name;

  /// Null = inherit default (`oneshot`).
  final String? playMode;
}

/// Next bank index for ◀/▶ chrome. Falls back to `0` when [activeIndex] is unset.
int cycleSamplerBankIndex({
  required int activeIndex,
  required int direction,
  required int length,
}) {
  if (length <= 0) {
    return -1;
  }
  final current = activeIndex >= 0 ? activeIndex : 0;
  return (current + direction + length * 8) % length;
}

class SamplerPads extends StatelessWidget {
  const SamplerPads({
    required this.slots,
    required this.banks,
    required this.activeBankId,
    required this.holdLike,
    required this.effectivePlayMode,
    required this.onPress,
    required this.onRelease,
    required this.onSelectBank,
    required this.onSaveBank,
    this.onAssign,
    this.disabled = false,
    super.key,
  });

  final List<SamplerSlot> slots;
  final List<SamplerBank> banks;
  final String? activeBankId;
  final bool holdLike;
  final String effectivePlayMode;
  final void Function(int slot, bool shift) onPress;
  final ValueChanged<int> onRelease;
  final ValueChanged<String> onSelectBank;
  final void Function(String bankId, String name, String? playMode) onSaveBank;
  final void Function(int slot, TrackDragPayload payload)? onAssign;
  final bool disabled;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final activeBankIndex = banks.indexWhere((b) => b.id == activeBankId);
    final activeBank = activeBankIndex >= 0 ? banks[activeBankIndex] : null;

    void cycleBank(int direction) {
      final next = cycleSamplerBankIndex(
        activeIndex: activeBankIndex,
        direction: direction,
        length: banks.length,
      );
      if (next < 0) {
        return;
      }
      onSelectBank(banks[next].id);
    }

    return PadGrid(
      bottomChrome: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 2, vertical: 2),
        child: Row(
          children: [
            _BankChromeButton(
              glyph: '◀',
              semanticLabel: 'Previous sampler bank',
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
                      style: theme.typography.mono.xs.copyWith(
                        fontWeight: FontWeight.w600,
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
                                  color: theme.colors.secondary.withValues(
                                    alpha: 0.5,
                                  ),
                                ),
                                child: Padding(
                                  padding: const EdgeInsets.symmetric(
                                    horizontal: 4,
                                    vertical: 1,
                                  ),
                                  child: Text(
                                    effectivePlayMode,
                                    style: theme.typography.body.xs.copyWith(
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
              glyph: '▶',
              semanticLabel: 'Next sampler bank',
              disabled: disabled || banks.length < 2,
              onPress: () => cycleBank(1),
            ),
            _BankChromeButton(
              glyph: '⚙',
              semanticLabel: 'Bank settings',
              disabled: disabled || activeBank == null,
              onPress: activeBank == null
                  ? null
                  : () => _openBankConfig(context, activeBank),
            ),
          ],
        ),
      ),
      children: [for (var slot = 0; slot < 8; slot++) _slotPad(theme, slot)],
    );
  }

  Widget _slotPad(FThemeData theme, int slot) {
    final sample = slot < slots.length ? slots[slot] : const SamplerSlot();
    final filled = sample.filled;
    final label = sample.label?.trim();
    final tooltip = filled
        ? (holdLike
              ? 'Pad ${slot + 1} — hold to play, shift+hold clear'
              : 'Pad ${slot + 1} — press trigger, shift+press clear')
        : 'Sampler pad ${slot + 1} (drop a track to assign)';

    final pad = HoldPadButton(
      disabled: disabled,
      accentSlot: filled ? slot : null,
      tooltip: tooltip,
      onBegin: () => onPress(slot, shiftKeyPressed()),
      onEnd: () => onRelease(slot),
      child: Column(
        mainAxisSize: .min,
        children: [
          Text(
            filled && label != null && label.isNotEmpty ? label : '${slot + 1}',
            overflow: TextOverflow.ellipsis,
            style: theme.typography.mono.xs.copyWith(
              fontWeight: FontWeight.w700,
            ),
          ),
          Text(
            filled && sample.durationMs != null
                ? formatDeckTimeTenth(sample.durationMs)
                : 'sample',
            overflow: TextOverflow.ellipsis,
            style: theme.typography.mono.xs,
          ),
        ],
      ),
    );

    final assign = onAssign;
    if (assign == null) {
      return pad;
    }
    return DropRegion(
      formats: const [Formats.plainText, Formats.fileUri],
      hitTestBehavior: HitTestBehavior.opaque,
      onDropOver: (event) {
        if (disabled) {
          return DropOperation.none;
        }
        for (final item in event.session.items) {
          if (parseTrackDragLocalData(item.localData) != null ||
              item.canProvide(Formats.plainText) ||
              item.canProvide(Formats.fileUri)) {
            return DropOperation.copy;
          }
        }
        return DropOperation.none;
      },
      onPerformDrop: (event) async {
        if (disabled) {
          return;
        }
        _performSamplerAssignDrop(event, slot, assign);
      },
      child: pad,
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
                      Text('Play mode', style: theme.typography.body.sm),
                      const SizedBox(height: 6),
                      Wrap(
                        spacing: 6,
                        runSpacing: 6,
                        children: [
                          for (final opt in kSamplerPlayModeOptions)
                            FButton(
                              variant: modeValue == opt ? .primary : .secondary,
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
    onSaveBank(
      bank.id,
      result.$1.trim().isEmpty ? bank.name : result.$1.trim(),
      result.$2,
    );
  }
}

void _performSamplerAssignDrop(
  PerformDropEvent event,
  int slot,
  void Function(int slot, TrackDragPayload payload) assign,
) {
  var assigned = false;
  void tryAssign(TrackDragPayload? payload) {
    if (assigned || payload == null) {
      return;
    }
    assigned = true;
    assign(slot, payload);
  }

  for (final item in event.session.items) {
    tryAssign(parseTrackDragLocalData(item.localData));
    if (assigned) {
      return;
    }
  }

  for (final item in event.session.items) {
    final reader = item.dataReader;
    if (reader == null) {
      continue;
    }
    if (item.canProvide(Formats.plainText)) {
      reader.getValue<String>(
        Formats.plainText,
        (text) => tryAssign(parseTrackDragPlainText(text)),
      );
    }
    if (item.canProvide(Formats.fileUri)) {
      reader.getValue<Uri>(Formats.fileUri, (uri) {
        if (uri == null) {
          return;
        }
        final path = pathFromDroppedUri(uri);
        if (isSupportedAudioPath(path)) {
          tryAssign(payloadFromOsPath(path));
        }
      });
    }
  }
}

class _BankChromeButton extends StatelessWidget {
  const _BankChromeButton({
    required this.glyph,
    required this.semanticLabel,
    required this.onPress,
    this.disabled = false,
  });

  final String glyph;
  final String semanticLabel;
  final VoidCallback? onPress;
  final bool disabled;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return Semantics(
      button: true,
      label: semanticLabel,
      enabled: !disabled,
      child: FButton(
        variant: .ghost,
        size: .xs,
        mainAxisSize: .min,
        onPress: disabled ? null : onPress,
        style: .delta(
          contentStyle: .delta(
            padding: .value(
              const EdgeInsets.symmetric(horizontal: 6, vertical: 4),
            ),
          ),
        ),
        child: ExcludeSemantics(
          child: Text(
            glyph,
            style: theme.typography.body.xs.copyWith(
              color: disabled
                  ? theme.colors.mutedForeground.withValues(alpha: 0.4)
                  : theme.colors.mutedForeground,
            ),
          ),
        ),
      ),
    );
  }
}
