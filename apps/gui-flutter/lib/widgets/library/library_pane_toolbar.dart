import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/utils/library_list_layout.dart';
import 'package:gui_flutter/widgets/shell/mixar_input.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// The filter/sort header both library panes share: a header-height bar with a
/// bottom border, a flat filter field filling it, and the pane's own trailing
/// controls on the right.
///
/// Kept here (rather than copied into both panes) so the header height, border
/// and field chrome have a single definition and cannot drift.
class LibraryPaneToolbar extends StatefulWidget {
  const new({
    required this.hint,
    required this.onChanged,
    required this.trailing,
    super.key,
  });

  /// Placeholder for the filter field.
  final String hint;

  final ValueChanged<String> onChanged;

  /// Controls drawn after the field (sort menu, density toggle, actions).
  final List<Widget> trailing;

  @override
  State<LibraryPaneToolbar> createState() => _LibraryPaneToolbarState();
}

class _LibraryPaneToolbarState extends State<LibraryPaneToolbar> {
  final FocusNode _focusNode = FocusNode();

  @override
  void initState() {
    super.initState();
    _focusNode.addListener(_onFocusChange);
  }

  @override
  void dispose() {
    _focusNode.removeListener(_onFocusChange);
    _focusNode.dispose();
    super.dispose();
  }

  void _onFocusChange() => setState(() {});

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    // A definite height is what lets the field band fill the header: with an
    // unbounded max height `Center` shrinks to the field, re-introducing a gap
    // above it. The clamp covers the whole toolbar — field *and* trailing
    // controls — so none of them can outgrow the fixed header; the tab bar is
    // icon-only, so it stays at its own height and the rows keep lining up.
    return MediaQuery.withClampedTextScaling(
      maxScaleFactor: 1.2,
      child: SizedBox(
        height: kLibraryHeaderHeight,
        child: DecoratedBox(
          key: kLibraryToolbarKey,
          decoration: BoxDecoration(
            border: Border(
              bottom: BorderSide(
                color: theme.colors.border,
                width: theme.style.borderWidth,
              ),
            ),
          ),
          child: Padding(
            // Inset by the border width so the band stops above the bottom
            // border instead of painting over it (`DecoratedBox` does not inset
            // its child by the border the way `Container` does).
            padding: EdgeInsets.only(right: 6, bottom: theme.style.borderWidth),
            child: Row(
              children: [
                Expanded(
                  child: ColoredBox(
                    color: libraryToolbarFieldColor(
                      theme,
                      focused: _focusNode.hasFocus,
                    ),
                    // Vertically centred; the field takes the full width
                    // (`ShadInput`'s row expands), so the visible band and the
                    // tappable field coincide.
                    child: Center(
                      child: MixarInput(
                        hint: widget.hint,
                        borderless: true,
                        focusNode: _focusNode,
                        padding: const EdgeInsets.symmetric(
                          horizontal: 12,
                          vertical: 6,
                        ),
                        onChanged: widget.onChanged,
                      ),
                    ),
                  ),
                ),
                const SizedBox(width: 6),
                ...widget.trailing,
              ],
            ),
          ),
        ),
      ),
    );
  }
}
