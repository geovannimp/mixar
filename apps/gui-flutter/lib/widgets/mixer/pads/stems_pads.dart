import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/models/pad_modes.dart';
import 'package:gui_flutter/widgets/mixer/pads/pad_button.dart';
import 'package:gui_flutter/widgets/mixer/pads/pad_face.dart';
import 'package:gui_flutter/widgets/mixer/pads/pad_grid.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// NI Stems default slot colors, matching the website stem pads.
///
/// Fixed metadata colors (not themed): keep in sync with website
/// `styles/global.css` `--stem-*` and `stem_atom.rs` `default_ni()`.
const kStemColors = <Color>[
  Color(0xFF009E73), // drums
  Color(0xFFD55E00), // bass
  Color(0xFFCC79A7), // other
  Color(0xFF56B4E9), // vocals
];

/// Website pad label: engine names are lowercase, display is title case.
String _titleCase(String name) => name[0].toUpperCase() + name.substring(1);

/// Stems pad mode: top row mute, bottom row solo (isolate).
///
/// Mirrors the website stem pads: hairline stem-tinted border at rest, solid
/// stem fill when lit, with a signal dot, uppercase mono action, and the stem
/// name in the display face.
class StemsPads extends StatelessWidget {
  const new({
    required this.stemMute,
    required this.stemIsolate,
    required this.stemsReady,
    required this.stemsGenerating,
    required this.onPress,
    this.disabled = false,
    super.key,
  });

  final List<bool> stemMute;
  final int? stemIsolate;
  final bool stemsReady;
  final bool stemsGenerating;
  final void Function(int slot) onPress;
  final bool disabled;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final inert = disabled || !stemsReady;
    final nameWidth = _nameSlotWidth(context, theme);

    return Column(
      children: [
        if (stemsGenerating)
          Padding(
            padding: const EdgeInsets.only(bottom: 4),
            child: Text(
              'STEMS GENERATING…',
              style: theme.typography.body.xs.copyWith(
                color: theme.colors.mutedForeground,
                fontWeight: FontWeight.w600,
                letterSpacing: 1.1,
              ),
            ),
          ),
        Expanded(
          child: PadGrid(
            children: [
              for (var slot = 0; slot < 8; slot++)
                _pad(theme, slot, inert: inert, nameWidth: nameWidth),
            ],
          ),
        ),
      ],
    );
  }

  /// Width of the widest stem name at the nominal type scale.
  ///
  /// Every pad sees a label of this exact width, so all eight names use one
  /// scale factor instead of each pad shrinking only its own wide name.
  double _nameSlotWidth(BuildContext context, MixarThemeData theme) {
    final scaler = MediaQuery.textScalerOf(context);
    final style = _nameStyle(theme);
    var width = 0.0;
    for (final stem in kStemNames) {
      final painter = TextPainter(
        text: TextSpan(text: _titleCase(stem), style: style),
        textDirection: TextDirection.ltr,
        textScaler: scaler,
      )..layout();
      if (painter.width > width) {
        width = painter.width;
      }
    }
    return width.ceilToDouble();
  }

  TextStyle _nameStyle(MixarThemeData theme) => theme.typography.display.md
      .copyWith(fontWeight: FontWeight.w600, height: 1.25, letterSpacing: -0.4);

  /// Rest/lit chrome for a stem pad; the 40% stem tint is shared with the dot.
  PadChrome _stemChrome(
    MixarThemeData theme,
    Color stemColor,
    Color nameColor, {
    required bool lit,
  }) {
    final dimStem = stemColor.withValues(alpha: 0.4);
    return PadChrome(
      // color-mix(in srgb, stem 40%, border) — tinted hairline at rest.
      border: lit ? stemColor : Color.alphaBlend(dimStem, theme.colors.border),
      fill: lit ? stemColor : theme.colors.card,
      foreground: nameColor,
      hoverBorder: stemColor,
      pressedFill: stemColor,
      pressedForeground: kPadOnAccentColor,
    );
  }

  Widget _pad(
    MixarThemeData theme,
    int slot, {
    required bool inert,
    required double nameWidth,
  }) {
    final stem = slot % 4;
    final isSoloPad = slot >= 4;
    final muted = stem < stemMute.length && stemMute[stem];
    final isolated = stemIsolate == stem;
    final lit = !inert && (isSoloPad ? isolated : muted);
    final stemColor = kStemColors[stem];
    final dimStem = stemColor.withValues(alpha: 0.4);

    final actionColor = lit ? kPadOnAccentColor.withValues(alpha: 0.8) : null;
    final nameColor = lit
        ? kPadOnAccentColor
        : inert
        ? theme.colors.mutedForeground
        : theme.colors.foreground;
    final dotColor = inert
        ? stemColor.withValues(alpha: 0.3)
        : lit
        ? kPadOnAccentColor
        : dimStem;

    return PadButton(
      disabled: inert,
      chrome: inert ? null : _stemChrome(theme, stemColor, nameColor, lit: lit),
      onPress: inert ? null : () => onPress(slot),
      fillChild: true,
      child: PadFace(
        action: isSoloPad ? 'SOLO' : 'MUTE',
        actionColor: actionColor,
        indicatorColor: dotColor,
        labelWidth: nameWidth,
        label: Text(
          _titleCase(kStemNames[stem]),
          textAlign: TextAlign.center,
          style: _nameStyle(theme).copyWith(color: nameColor),
        ),
      ),
    );
  }
}
