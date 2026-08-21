import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';

/// Header `--- Label ---` that shows or hides [child].
///
/// Set [fillRemaining] when this is a direct [Column] child that should take
/// leftover height while open (inner [child] is then wrapped in [Expanded]).
class CollapsibleSection extends StatefulWidget {
  const CollapsibleSection({
    required this.label,
    required this.child,
    this.initiallyOpen = true,
    this.fillRemaining = false,
    this.expandHeader = true,
    this.onOpenChanged,
    super.key,
  });

  final String label;
  final Widget child;
  final bool initiallyOpen;
  final bool fillRemaining;
  final bool expandHeader;
  final ValueChanged<bool>? onOpenChanged;

  @override
  State<CollapsibleSection> createState() => _CollapsibleSectionState();
}

class _CollapsibleSectionState extends State<CollapsibleSection> {
  late var _open = widget.initiallyOpen;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final column = Column(
      mainAxisSize: widget.fillRemaining && _open ? .max : .min,
      children: [
        Row(
          mainAxisSize: widget.expandHeader ? .max : .min,
          spacing: 4,
          children: [
            if (widget.expandHeader) const Expanded(child: FDivider()),
            FButton(
              key: ValueKey('${widget.label.toLowerCase()}-panel-toggle'),
              variant: .ghost,
              size: .xs,
              mainAxisSize: .min,
              onPress: () {
                setState(() => _open = !_open);
                widget.onOpenChanged?.call(_open);
              },
              child: Text(
                widget.label,
                style: theme.typography.body.xs2.copyWith(
                  fontWeight: .w600,
                  letterSpacing: 1.6,
                  color: theme.colors.mutedForeground,
                ),
              ),
            ),
            if (widget.expandHeader) const Expanded(child: FDivider()),
          ],
        ),
        if (_open)
          widget.fillRemaining ? Expanded(child: widget.child) : widget.child,
      ],
    );
    if (!widget.fillRemaining) {
      return column;
    }
    return Flexible(
      flex: _open ? 1 : 0,
      fit: _open ? .tight : .loose,
      child: column,
    );
  }
}
