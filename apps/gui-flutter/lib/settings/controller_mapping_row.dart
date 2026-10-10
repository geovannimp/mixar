import 'package:flutter/widgets.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/library/library_list_chrome.dart'
    show MetaPill, kMetaPillGap;
import 'package:gui_flutter/shell/app_button.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:gui_flutter/src/rust/api/controller.dart';

/// Below this row width the trailing controls drop under the mapping title
/// instead of competing with it for horizontal space.
const _kStackBreakpoint = 480.0;

class ControllerMappingRow extends StatelessWidget {
  const new({
    required this.mapping,
    required this.attached,
    required this.trusted,
    required this.attachBusy,
    required this.trustBusy,
    required this.onToggleAttach,
    required this.onToggleTrust,
    required this.onUpdate,
    super.key,
  });

  final ControllerMappingInfo mapping;
  final bool attached;
  final bool trusted;
  final bool attachBusy;
  final bool trustBusy;
  final ValueChanged<bool> onToggleAttach;
  final ValueChanged<bool> onToggleTrust;
  final VoidCallback onUpdate;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final l10n = AppLocalizations.of(context)!;
    final name = [
      mapping.vendorName,
      mapping.productName,
    ].where((s) => s.isNotEmpty).join(' ');
    final version = mapping.version;
    // Metadata as the library track row's chips: id, device id, version, and
    // an accent "Attached" state. The pill's default fill is the row surface
    // (`card`), which is invisible on this settings card, so use the page
    // background fill + border (matching the library's status pill).
    MetaPill chip(String text, {Color? textColor, FontWeight? fontWeight}) =>
        MetaPill(
          text: text,
          textColor: textColor,
          fontWeight: fontWeight ?? FontWeight.w500,
          backgroundColor: theme.colors.background,
          borderColor: theme.colors.border,
        );
    final metaPills = <Widget>[
      chip(mapping.id),
      chip(mapping.deviceId),
      if (version != null) chip('v$version'),
      if (attached)
        chip(
          l10n.settingsControllersAttached,
          textColor: theme.colors.primary,
          fontWeight: FontWeight.w600,
        ),
    ];

    final leading = Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      spacing: 2,
      children: [
        Text(
          name,
          style: theme.typography.body.sm.copyWith(fontWeight: FontWeight.w600),
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
        ),
        if (metaPills.isNotEmpty)
          Wrap(
            spacing: kMetaPillGap,
            runSpacing: kMetaPillGap,
            children: metaPills,
          ),
      ],
    );

    final trailing = Wrap(
      spacing: 8,
      runSpacing: 8,
      crossAxisAlignment: WrapCrossAlignment.center,
      children: [
        if (mapping.updateAvailable)
          Text(
            l10n.settingsControllersUpdateAvailable,
            style: theme.typography.body.xs.copyWith(
              color: theme.colors.primary,
              fontWeight: FontWeight.w600,
            ),
          ),
        AppButton(
          variant: .outline,
          size: .sm,
          mainAxisSize: .min,
          onPress: attachBusy ? null : onUpdate,
          child: Text(l10n.commonUpdate),
        ),
        _ToggleButton(
          label: l10n.settingsControllersTrust,
          on: trusted,
          enabled: !trustBusy,
          semanticsLabel: l10n.settingsControllersTrustSemantics(name),
          onPress: () => onToggleTrust(!trusted),
        ),
        _ToggleButton(
          label: l10n.settingsControllersAttach,
          on: attached,
          enabled: !attachBusy,
          semanticsLabel: l10n.settingsControllersEnableSemantics(name),
          onPress: () => onToggleAttach(!attached),
        ),
      ],
    );

    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 6),
      child: LayoutBuilder(
        builder: (context, constraints) {
          // Stack on narrow panes so the intrinsically-sized controls can wrap
          // instead of overflowing the row.
          if (constraints.maxWidth < _kStackBreakpoint) {
            return Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              spacing: 8,
              children: [leading, trailing],
            );
          }
          return Row(
            spacing: 12,
            children: [
              Expanded(child: leading),
              trailing,
            ],
          );
        },
      ),
    );
  }
}

/// Latched outline button for Trust / Attach (deck-panel toggle styling).
class _ToggleButton extends StatelessWidget {
  const new({
    required this.label,
    required this.on,
    required this.enabled,
    required this.semanticsLabel,
    required this.onPress,
  });

  final String label;
  final bool on;
  final bool enabled;
  final String semanticsLabel;
  final VoidCallback onPress;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return AppButton(
      variant: .outline,
      size: .sm,
      mainAxisSize: .min,
      selected: on,
      backgroundColor: on ? theme.colors.primaryTint : null,
      onPress: enabled ? onPress : null,
      semanticsLabel: semanticsLabel,
      child: Text(label),
    );
  }
}
