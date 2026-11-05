import 'dart:math' as math;

import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/theme/mixar_accent.dart';
import 'package:gui_flutter/core/theme/mixar_colors.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/onboarding/onboarding_ui.dart';
import 'package:gui_flutter/widgets/settings/settings_accent_picker.dart';
import 'package:gui_flutter/widgets/settings/settings_labels.dart';
import 'package:gui_flutter/widgets/shell/m_tappable.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Step 2: theme, accent and language — chosen on live previews.
///
/// Every tap repaints the whole window: `Application` reads the draft, so the
/// preview is the real app, not a mock.
class OnboardingAppearanceStep extends StatelessWidget {
  const new({required this.draft, required this.onChanged, super.key});

  final AppSettings draft;
  final ValueChanged<AppSettings> onChanged;

  static const List<ThemeSetting> _themes = [
    ThemeSetting.system,
    ThemeSetting.light,
    ThemeSetting.dark,
  ];

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final palette = accentPaletteFromSettings(draft.accentPalette);
    return OnboardingBody(
      eyebrow: l10n.onboardingKickerAppearance,
      title: l10n.onboardingAppearanceTitle,
      description: l10n.onboardingAppearanceBody,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        spacing: 28,
        children: [
          Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            spacing: 10,
            children: [
              OnboardingGroupLabel(label: l10n.settingsUiTheme),
              Row(
                spacing: 12,
                children: [
                  for (final theme in _themes)
                    Expanded(
                      child: _ThemeTile(
                        value: theme,
                        palette: palette,
                        selected: draft.theme == theme,
                        onPress: () =>
                            onChanged(copyAppSettings(draft, theme: theme)),
                      ),
                    ),
                ],
              ),
            ],
          ),
          Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            spacing: 10,
            children: [
              OnboardingGroupLabel(label: l10n.settingsUiAccentColor),
              SettingsAccentPicker(
                value: draft.accentPalette,
                onChanged: (accent) =>
                    onChanged(copyAppSettings(draft, accentPalette: accent)),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

/// Theme option with a miniature of the app painted in that brightness.
class _ThemeTile extends StatelessWidget {
  const new({
    required this.value,
    required this.palette,
    required this.selected,
    required this.onPress,
  });

  final ThemeSetting value;
  final MixarAccentPalette palette;
  final bool selected;
  final VoidCallback onPress;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final l10n = AppLocalizations.of(context)!;
    return MTappable(
      onPress: onPress,
      selected: selected,
      semanticsLabel: themeSettingLabel(l10n, value),
      builder: (_, state) {
        final colors = theme.colors;
        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            // Foreground: the wireframe fills its box, so a background border
            // would be painted over by the preview.
            DecoratedBox(
              position: DecorationPosition.foreground,
              decoration: BoxDecoration(
                border: Border.all(
                  color: selected
                      ? colors.primary
                      : state.hovered || state.focused
                      ? colors.accent
                      : colors.border,
                  width: selected ? 2 : theme.style.borderWidth,
                ),
              ),
              child: SizedBox(
                height: 104,
                child: Stack(
                  fit: StackFit.expand,
                  children: [
                    _ThemePreview(value: value, palette: palette),
                    if (selected)
                      const Positioned(
                        top: 0,
                        right: 0,
                        child: _SelectedBadge(),
                      ),
                  ],
                ),
              ),
            ),
            const SizedBox(height: 8),
            // The tile's own semantics label carries the name; the visible
            // caption must not be read twice.
            ExcludeSemantics(
              child: Text(
                themeSettingLabel(l10n, value),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: theme.typography.body.sm.copyWith(
                  color: selected ? colors.foreground : colors.mutedForeground,
                  fontWeight: selected ? FontWeight.w600 : FontWeight.w400,
                ),
              ),
            ),
          ],
        );
      },
    );
  }
}

/// Wireframe of the app painted in the palette at one brightness.
///
/// A scaled-down mixer: brand strip, waveform lane, two decks with a mixer
/// between them, library band. `System default` paints one wireframe whose left
/// half is light and right half dark, so it reads as one window following the
/// OS rather than two separate themes.
class _ThemePreview extends StatelessWidget {
  const new({required this.value, required this.palette});

  final ThemeSetting value;
  final MixarAccentPalette palette;

  @override
  Widget build(BuildContext context) {
    final light = MixarThemeData.of(palette, Brightness.light).colors;
    final dark = MixarThemeData.of(palette, Brightness.dark).colors;
    return CustomPaint(
      painter: switch (value) {
        ThemeSetting.light => _WireframePainter(left: light, right: light),
        ThemeSetting.dark => _WireframePainter(left: dark, right: dark),
        ThemeSetting.system => _WireframePainter(left: light, right: dark),
      },
    );
  }
}

/// One app wireframe; [left] paints the left half, [right] the right half.
class _WireframePainter extends CustomPainter {
  const new({required this.left, required this.right});

  final MixarColors left;
  final MixarColors right;

  @override
  void paint(Canvas canvas, Size size) {
    final half = size.width / 2;
    final split = left != right;
    if (!split) {
      _scene(canvas, size, left);
      return;
    }
    _paintHalf(canvas, size, Rect.fromLTWH(0, 0, half, size.height), left);
    _paintHalf(canvas, size, Rect.fromLTWH(half, 0, half, size.height), right);
    canvas..drawRect(
      Rect.fromLTWH(half - 0.5, 0, 1, size.height),
      Paint()..color = left.border,
    );
  }

  /// [scene] clipped to [rect], with the canvas stack restored afterwards.
  void _paintHalf(Canvas canvas, Size size, Rect rect, MixarColors colors) {
    canvas
      ..save()
      ..clipRect(rect);
    _scene(canvas, size, colors);
    canvas.restore();
  }

  void _scene(Canvas canvas, Size size, MixarColors colors) {
    final w = size.width;
    final h = size.height;
    final hairline = Paint()..color = colors.border;
    final card = Paint()..color = colors.card;
    final accent = Paint()..color = colors.accent;

    canvas.drawRect(Offset.zero & size, Paint()..color = colors.background);

    // Brand strip.
    final headerH = h * 0.15;
    canvas
      ..drawRect(Rect.fromLTWH(0, 0, w, headerH), card)
      ..drawRect(Rect.fromLTWH(6, headerH * 0.42, 14, 2.5), accent)
      ..drawRect(Rect.fromLTWH(0, headerH, w, 1), hairline);

    // Waveform lane. Mirrored about its centre line and dense, like the
    // analysed audio in the app: a sum of slow sines for the envelope and a
    // small alternating jitter for the transient detail.
    final waveTop = headerH + 4;
    final waveH = h * 0.14;
    final centre = waveTop + waveH / 2;
    const bars = 64;
    final step = w / bars;
    for (var i = 0; i < bars; i++) {
      final t = i / bars;
      final envelope = 0.18 + 0.82 * math.pow(math.sin(t * math.pi), 0.8);
      final detail =
          0.55 +
          0.25 * math.sin(t * math.pi * 6.2) +
          0.2 * math.sin(t * math.pi * 17.3 + 1.1);
      final half = (waveH / 2) * envelope * detail.clamp(0.15, 1.0);
      canvas.drawRect(
        Rect.fromLTWH(i * step, centre - half, step * 0.62, half * 2),
        accent,
      );
    }
    // Playhead, so the lane reads as playing audio rather than a bar chart.
    canvas.drawRect(
      Rect.fromLTWH(w * 0.46, waveTop, 1, waveH),
      Paint()..color = colors.foreground.withValues(alpha: 0.55),
    );

    // Deck row: deck | mixer | deck.
    final rowTop = waveTop + waveH + 4;
    final rowH = h * 0.42;
    final deckW = w * 0.36;
    final mixerX = deckW + 6;
    final mixerW = w - deckW * 2 - 12;
    for (final x in [4.0, w - deckW - 4]) {
      final rect = Rect.fromLTWH(x, rowTop, deckW, rowH);
      canvas
        ..drawRect(rect, card)
        ..drawRect(
          rect,
          Paint()
            ..style = PaintingStyle.stroke
            ..strokeWidth = 1
            ..color = colors.border,
        );
      final stripe = Paint()..color = colors.selection;
      final caption = Paint()
        ..color = colors.mutedForeground.withValues(alpha: 0.5);
      canvas
        ..drawRect(
          Rect.fromLTWH(x + 5, rowTop + rowH - 6, deckW - 10, 2),
          stripe,
        )
        ..drawRect(Rect.fromLTWH(x + 5, rowTop + 5, 10, 2), caption);
    }
    // Mixer: two channel faders and the crossfader.
    canvas.drawRect(
      Rect.fromLTWH(mixerX, rowTop, mixerW, rowH),
      Paint()..color = colors.muted,
    );
    for (final x in [mixerX + mixerW * 0.3, mixerX + mixerW * 0.7]) {
      canvas
        ..drawRect(Rect.fromLTWH(x - 0.5, rowTop + 4, 1, rowH - 14), hairline)
        ..drawRect(Rect.fromLTWH(x - 2.5, rowTop + rowH * 0.35, 5, 6), accent);
    }
    canvas.drawRect(
      Rect.fromLTWH(mixerX + 3, rowTop + rowH - 5, mixerW - 6, 2),
      hairline,
    );

    // Library band.
    final libTop = rowTop + rowH + 4;
    canvas.drawRect(
      Rect.fromLTWH(0, libTop, w, h - libTop),
      Paint()..color = colors.muted,
    );
    for (var i = 0; i < 3; i++) {
      canvas.drawRect(
        Rect.fromLTWH(6 + i * (w / 3.4), libTop + 4, w / 5.2, 2),
        Paint()..color = colors.mutedForeground.withValues(alpha: 0.35),
      );
    }
    canvas.drawRect(Rect.fromLTWH(0, libTop, w, 1), hairline);
  }

  @override
  bool shouldRepaint(covariant _WireframePainter old) =>
      old.left != left || old.right != right;
}

/// Corner tick on the selected theme tile.
class _SelectedBadge extends StatelessWidget {
  const new();

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return SizedBox.square(
      dimension: 16,
      child: ColoredBox(
        color: theme.colors.primary,
        child: Icon(
          LucideIcons.check,
          size: 11,
          color: theme.colors.primaryForeground,
        ),
      ),
    );
  }
}
