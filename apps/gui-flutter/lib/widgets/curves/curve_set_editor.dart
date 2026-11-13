import 'dart:math' as math;

import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/models/curve_editor.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/src/rust/api/event_mirrors.dart';
import 'package:gui_flutter/widgets/curves/curve_chart.dart';
import 'package:gui_flutter/widgets/settings/settings_field.dart';
import 'package:gui_flutter/widgets/settings/settings_select.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/mixar_slider.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Ordered members, morph control, and A/B preview for one curve set.
class CurveSetEditor extends StatefulWidget {
  const new({
    required this.set,
    required this.library,
    required this.onChanged,
    super.key,
  });

  final CurveSetSpec set;
  final CurveLibrarySpec library;
  final ValueChanged<CurveSetSpec> onChanged;

  @override
  State<CurveSetEditor> createState() => _CurveSetEditorState();
}

class _CurveSetEditorState extends State<CurveSetEditor> {
  var _morph = 0.0;

  List<String> get _availableMembers {
    final used = widget.set.members.map((member) => member.id).toSet();
    return [
      for (final id in kCrossfaderPresetIds)
        if (!used.contains(id)) id,
      for (final group in widget.library.groups)
        if (group.curves.length == 2 && !used.contains(group.id)) group.id,
    ];
  }

  String _labelFor(String id) {
    final reference = CurveRefSpec(id: id);
    return curveSetMemberLabel(reference, widget.library);
  }

  void _addMember(String id) {
    widget.onChanged(
      copyCurveSet(
        widget.set,
        members: [
          ...widget.set.members,
          CurveRefSpec(id: id),
        ],
      ),
    );
  }

  void _removeMember(int index) {
    if (widget.set.members.length <= kMinCurveSetMembers) {
      return;
    }
    final members = [...widget.set.members]..removeAt(index);
    widget.onChanged(copyCurveSet(widget.set, members: members));
  }

  void _moveMember(int index, int delta) {
    final target = index + delta;
    if (target < 0 || target >= widget.set.members.length) {
      return;
    }
    final members = [...widget.set.members];
    final member = members.removeAt(index);
    members.insert(target, member);
    widget.onChanged(copyCurveSet(widget.set, members: members));
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final l10n = AppLocalizations.of(context)!;
    final error = curveSetValidationError(widget.set, widget.library);
    final available = _availableMembers;
    const samples = 96;
    final channels = math.max(
      1,
      widget.set.members.isEmpty
          ? 1
          : curveRefChannels(widget.set.members.first, widget.library),
    );
    final series = [
      for (var channel = 0; channel < channels; channel++)
        CurveSeries(
          values: [
            for (var sample = 0; sample < samples; sample++)
              _sampleCurveSet(
                widget.set,
                widget.library,
                _morph,
                sample / (samples - 1),
                channel,
              ),
          ],
          color: channel == 0 ? theme.colors.primary : theme.colors.cueOn,
          strokeWidth: 1.8,
          filled: channel == 0,
        ),
    ];

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      spacing: 14,
      children: [
        SettingsField(
          label: l10n.curvesMembers,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            spacing: 8,
            children: [
              for (var index = 0; index < widget.set.members.length; index++)
                _MemberRow(
                  label: curveSetMemberLabel(
                    widget.set.members[index],
                    widget.library,
                  ),
                  id: widget.set.members[index].id,
                  first: index == 0,
                  last: index == widget.set.members.length - 1,
                  canRemove: widget.set.members.length > kMinCurveSetMembers,
                  onUp: () => _moveMember(index, -1),
                  onDown: () => _moveMember(index, 1),
                  onRemove: () => _removeMember(index),
                ),
              if (available.isNotEmpty)
                SettingsSelect<String>(
                  dialogTitle: l10n.curvesAddMember,
                  value: available.first,
                  options: available,
                  labelBuilder: _labelFor,
                  onChanged: _addMember,
                ),
            ],
          ),
        ),
        if (error != null)
          Text(
            error,
            style: theme.typography.body.xs.copyWith(
              color: theme.colors.destructive,
            ),
          )
        else
          Text(
            l10n.curvesSetHint,
            style: theme.typography.body.xs.copyWith(
              color: theme.colors.mutedForeground,
            ),
          ),
        Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          spacing: 4,
          children: [
            Row(
              children: [
                Expanded(
                  child: Text(
                    l10n.curvesMorphPreview,
                    style: theme.typography.body.xs.copyWith(
                      color: theme.colors.mutedForeground,
                      fontWeight: FontWeight.w700,
                      letterSpacing: 0.8,
                    ),
                  ),
                ),
                Text(
                  _morph.toStringAsFixed(2),
                  style: theme.typography.body.xs.copyWith(
                    fontFeatures: const [FontFeature.tabularFigures()],
                  ),
                ),
              ],
            ),
            MixarSlider(
              value: _morph,
              defaultValue: 0,
              semanticFormatterCallback: (value) => value.toStringAsFixed(2),
              onChanged: (value) => setState(() => _morph = value),
            ),
            CurveChart(
              series: series,
              semanticLabel: l10n.curvesMorphSemantics(widget.set.displayName),
            ),
          ],
        ),
      ],
    );
  }
}

double _sampleCurveSet(
  CurveSetSpec set,
  CurveLibrarySpec library,
  double morph,
  double x,
  int channel,
) {
  final values = evaluateCurveSet(set, library, morph, x);
  return channel < values.length ? values[channel] : 0;
}

class _MemberRow extends StatelessWidget {
  const new({
    required this.label,
    required this.id,
    required this.first,
    required this.last,
    required this.canRemove,
    required this.onUp,
    required this.onDown,
    required this.onRemove,
  });

  final String label;
  final String id;
  final bool first;
  final bool last;
  final bool canRemove;
  final VoidCallback onUp;
  final VoidCallback onDown;
  final VoidCallback onRemove;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final l10n = AppLocalizations.of(context)!;
    return DecoratedBox(
      decoration: BoxDecoration(
        border: Border.all(
          color: theme.colors.border,
          width: theme.style.borderWidth,
        ),
        borderRadius: theme.style.borderRadius.md,
      ),
      child: Padding(
        padding: const EdgeInsets.fromLTRB(10, 6, 4, 6),
        child: Row(
          children: [
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    label,
                    style: theme.typography.body.sm.copyWith(
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  Text(
                    id,
                    style: theme.typography.body.xs.copyWith(
                      color: theme.colors.mutedForeground,
                    ),
                  ),
                ],
              ),
            ),
            AppButton.icon(
              size: .sm,
              variant: .ghost,
              semanticsLabel: l10n.curvesMoveUp,
              onPress: first ? null : onUp,
              child: const Icon(LucideIcons.arrowUp, size: 16),
            ),
            AppButton.icon(
              size: .sm,
              variant: .ghost,
              semanticsLabel: l10n.curvesMoveDown,
              onPress: last ? null : onDown,
              child: const Icon(LucideIcons.arrowDown, size: 16),
            ),
            AppButton.icon(
              size: .sm,
              variant: .ghost,
              semanticsLabel: l10n.curvesDelete,
              onPress: canRemove ? onRemove : null,
              child: const Icon(LucideIcons.trash2, size: 16),
            ),
          ],
        ),
      ),
    );
  }
}
