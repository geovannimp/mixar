import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/theme/mixar_accent.dart';
import 'package:gui_flutter/core/theme/mixar_colors.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_preview_block.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// The smallest readable picture of the app: a title strip, two decks around
/// the mixer, and the library band, painted in the theme's own tokens.
///
/// `System default` paints one window whose left half is light and right half
/// dark, so it reads as one app following the OS rather than two themes.
class ThemeSwatch extends StatelessWidget {
  const new({required this.value, required this.palette, super.key});

  final ThemeSetting value;
  final MixarAccentPalette palette;

  @override
  Widget build(BuildContext context) {
    final light = MixarThemeData.of(palette, Brightness.light).colors;
    final dark = MixarThemeData.of(palette, Brightness.dark).colors;
    return switch (value) {
      ThemeSetting.light => _MiniApp(colors: light),
      ThemeSetting.dark => _MiniApp(colors: dark),
      ThemeSetting.system => Stack(
        fit: StackFit.expand,
        children: [
          ClipRect(
            clipper: const _Half(left: true),
            child: _MiniApp(colors: light),
          ),
          ClipRect(
            clipper: const _Half(left: false),
            child: _MiniApp(colors: dark),
          ),
          Positioned.fill(
            child: Align(
              child: SizedBox(
                width: 1,
                height: double.infinity,
                child: ColoredBox(color: light.border),
              ),
            ),
          ),
        ],
      ),
    };
  }
}

class _MiniApp extends StatelessWidget {
  const new({required this.colors});

  final MixarColors colors;

  @override
  Widget build(BuildContext context) {
    Widget bar(double height, Color color) => SizedBox(
      height: height,
      child: SettingsPreviewBlock(color: color),
    );

    return DecoratedBox(
      decoration: BoxDecoration(
        color: colors.background,
        border: Border.all(color: colors.border),
        borderRadius: BorderRadius.circular(2),
      ),
      child: Padding(
        padding: const EdgeInsets.all(3),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          spacing: 2,
          children: [
            // Title strip with the brand accent at its left.
            Container(
              height: 6,
              alignment: Alignment.centerLeft,
              padding: const EdgeInsets.only(left: 3),
              decoration: BoxDecoration(
                color: colors.card,
                borderRadius: BorderRadius.circular(1),
              ),
              child: SizedBox(
                width: 10,
                height: 2,
                child: SettingsPreviewBlock(color: colors.primary),
              ),
            ),
            Expanded(
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                spacing: 2,
                children: [
                  Expanded(
                    flex: 3,
                    child: SettingsPreviewBlock(color: colors.card),
                  ),
                  Expanded(
                    flex: 2,
                    child: SettingsPreviewBlock(color: colors.muted),
                  ),
                  Expanded(
                    flex: 3,
                    child: SettingsPreviewBlock(color: colors.card),
                  ),
                ],
              ),
            ),
            bar(5, colors.muted),
          ],
        ),
      ),
    );
  }
}

/// Clips a full-size [_MiniApp] to one half of the System tile.
class _Half extends CustomClipper<Rect> {
  const new({required this.left});

  final bool left;

  @override
  Rect getClip(Size size) => left
      ? Rect.fromLTWH(0, 0, size.width / 2, size.height)
      : Rect.fromLTWH(size.width / 2, 0, size.width / 2, size.height);

  @override
  bool shouldReclip(covariant _Half oldClipper) => oldClipper.left != left;
}
