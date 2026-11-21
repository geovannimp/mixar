import 'package:gui_flutter/core/utils/desktop.dart';
import 'package:gui_flutter/widgets/shell/m_tappable.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';
import 'package:window_manager/window_manager.dart';

import 'window_actions.dart';

/// Diameter of one traffic light.
const double _kLightSize = 14;

/// Gap between two lights.
const double _kLightGap = 8;

/// Box a light's glyph is drawn in while the pointer is over the cluster.
///
/// Hand-drawn rather than an icon font: at this size a font glyph lands on a
/// sub-pixel stroke and reads as a smudge, where macOS paints a crisp mark.
const double _kGlyphSize = 9;

/// Stroke weight and arm length of those marks, in logical pixels.
const double _kGlyphStroke = 1.6;
const double _kGlyphArm = 2.8;

/// Grey the three lights turn into while the window is not focused.
const _kInactiveOnDark = Color(0xFF4C4E51);
const _kInactiveOnLight = Color(0xFFD0D2D5);

/// Glyph ink: black at half strength over every light, the way macOS draws it.
const _kGlyphInk = Color(0x9E000000);

/// Close / minimise / zoom in the header's leading corner, drawn by the app.
///
/// The macOS build hides AppKit's own buttons (`windowButtonVisibility` in
/// `main.dart`), so every macOS-layout host owns that corner: the cluster is
/// the window's close/minimise/zoom, and the Linux debug preview draws the
/// same three circles and wires them the same way, so clicking one behaves
/// exactly as it will on a Mac. Both centre the cluster in
/// [kMacOSWindowControlsInset] on both axes.
///
/// It follows macOS's own cues: grey lights while the window is not focused,
/// and the glyphs appear only while the pointer is over the cluster.
class MacOSTitleBarChrome extends StatelessWidget {
  const new({required this.child, super.key});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    if (!usesMacOSTitleBarLayout) {
      return child;
    }
    // Passthrough keeps the header's own height and constraints: the lights
    // paint over it, they do not size it.
    return Stack(
      fit: StackFit.passthrough,
      children: [
        child,
        Positioned(
          left: 0,
          top: 0,
          bottom: 0,
          width: kMacOSWindowControlsInset,
          child: _TrafficLights(preview: isMacOSChromePreview),
        ),
      ],
    );
  }
}

/// The cluster itself: three live window controls.
///
/// [preview] only adds the label that tells a Linux debug run's cluster apart
/// from a Mac's own; the lights behave the same in both.
class _TrafficLights extends StatefulWidget {
  const new({required this.preview});

  final bool preview;

  @override
  State<_TrafficLights> createState() => _TrafficLightsState();
}

class _TrafficLightsState extends State<_TrafficLights> {
  ListenerId? _listener;
  var _hovered = false;
  var _focused = true;

  @override
  void initState() {
    super.initState();
    final window = _window;
    // Widget tests have no native host to ask; the override stands in, and a
    // headless run keeps the focused look rather than failing over decoration.
    _focused = debugOverrideWindowFocused ?? window?.isFocused ?? true;
    if (window == null) {
      return;
    }
    _listener = WindowManager.instance.addListener((event) {
      final focused = switch (event) {
        WindowFocusedEvent() => true,
        WindowBlurredEvent() => false,
        _ => null,
      };
      if (focused != null && mounted && focused != _focused) {
        setState(() => _focused = focused);
      }
    });
  }

  @override
  void dispose() {
    final listener = _listener;
    if (listener != null) {
      WindowManager.instance.removeListener(listener);
    }
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    // macOS greys the lights out on an unfocused window; the grey has to read
    // against the header fill, which follows the app theme.
    final dark =
        MixarTheme.maybeOf(context)?.colors.brightness != Brightness.light;
    final inactive = dark ? _kInactiveOnDark : _kInactiveOnLight;
    // Glyphs belong to the focused window only, as they do on macOS.
    final glyphs = _focused && _hovered;

    final cluster = Center(
      child: MouseRegion(
        onEnter: (_) => setState(() => _hovered = true),
        onExit: (_) => setState(() => _hovered = false),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            _WindowLight(
              color: const Color(0xFFFF5F57),
              inactiveColor: inactive,
              active: _focused,
              glyph: glyphs ? _GlyphKind.close : null,
              semanticsLabel: 'Close',
              onPress: closeApp,
            ),
            const SizedBox(width: _kLightGap),
            _WindowLight(
              color: const Color(0xFFFEBC2E),
              inactiveColor: inactive,
              active: _focused,
              glyph: glyphs ? _GlyphKind.minimize : null,
              semanticsLabel: 'Minimize',
              onPress: _toggleMinimized,
            ),
            const SizedBox(width: _kLightGap),
            _WindowLight(
              color: const Color(0xFF28C840),
              inactiveColor: inactive,
              active: _focused,
              glyph: glyphs ? _GlyphKind.zoom : null,
              semanticsLabel: 'Zoom',
              onPress: _toggleMaximized,
            ),
          ],
        ),
      ),
    );
    if (!widget.preview) {
      return cluster;
    }
    return Semantics(
      container: true,
      label: 'macOS title-bar layout preview',
      child: cluster,
    );
  }
}

/// The host window, or null where there is none (widget tests, headless runs).
Window? get _window => WindowManager.instance.getCurrent();

/// Minimise, or bring the window back when it is already down.
void _toggleMinimized() {
  final window = _window;
  if (window == null) {
    return;
  }
  if (window.isMinimized) {
    window.restore();
  } else {
    window.minimize();
  }
}

/// Zoom, or undo it — the same toggle the app's other window buttons offer.
void _toggleMaximized() {
  final window = _window;
  if (window == null) {
    return;
  }
  if (window.isMaximized) {
    window.unmaximize();
  } else {
    window.maximize();
  }
}

/// One live light: press dims it, an unfocused window greys it, and a hovered
/// cluster shows its glyph.
class _WindowLight extends StatelessWidget {
  const new({
    required this.color,
    required this.inactiveColor,
    required this.active,
    required this.glyph,
    required this.semanticsLabel,
    required this.onPress,
  });

  final Color color;
  final Color inactiveColor;
  final bool active;
  final _GlyphKind? glyph;
  final String semanticsLabel;
  final VoidCallback onPress;

  @override
  Widget build(BuildContext context) => MTappable(
    semanticsLabel: semanticsLabel,
    onPress: onPress,
    builder: (context, state) => _LightDot(
      color: active ? color : inactiveColor,
      pressed: state.pressed,
      glyph: active ? glyph : null,
    ),
  );
}

/// The mark macOS paints on a hovered light.
enum _GlyphKind {
  close(Key('macos-close-glyph')),
  minimize(Key('macos-minimize-glyph')),
  zoom(Key('macos-zoom-glyph'));

  new(this.key);

  /// Stable handle for tests, which cannot reach a private widget type.
  final Key key;
}

/// One light. macOS's own red/yellow/green, not theme tokens, so the corner
/// reads the same in every app theme.
class _LightDot extends StatelessWidget {
  const new({required this.color, this.pressed = false, this.glyph});

  final Color color;
  final bool pressed;
  final _GlyphKind? glyph;

  @override
  Widget build(BuildContext context) => SizedBox.square(
    dimension: _kLightSize,
    child: DecoratedBox(
      decoration: BoxDecoration(
        color: pressed
            ? Color.alphaBlend(const Color(0x40000000), color)
            : color,
        shape: BoxShape.circle,
      ),
      child: glyph == null
          ? null
          : Center(
              child: CustomPaint(
                key: glyph!.key,
                size: const Size.square(_kGlyphSize),
                painter: _GlyphPainter(glyph!),
              ),
            ),
    ),
  );
}

/// Close's cross, minimise's bar and zoom's plus, drawn at the weight macOS
/// uses so they read at 14 logical pixels.
class _GlyphPainter extends CustomPainter {
  const new(this.kind);

  final _GlyphKind kind;

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = _kGlyphInk
      ..strokeWidth = _kGlyphStroke
      ..strokeCap = StrokeCap.round
      ..style = PaintingStyle.stroke;
    final centre = size.center(Offset.zero);
    // Segments around the centre: the cross for close, one bar for minimise,
    // two for zoom. Same weight everywhere, so the three read as a set.
    const closeReach = 2.6;
    final segments = switch (kind) {
      _GlyphKind.close => const [
        (Offset(-closeReach, -closeReach), Offset(closeReach, closeReach)),
        (Offset(closeReach, -closeReach), Offset(-closeReach, closeReach)),
      ],
      _GlyphKind.minimize => const [
        (Offset(-_kGlyphArm, 0), Offset(_kGlyphArm, 0)),
      ],
      _GlyphKind.zoom => const [
        (Offset(-_kGlyphArm, 0), Offset(_kGlyphArm, 0)),
        (Offset(0, -_kGlyphArm), Offset(0, _kGlyphArm)),
      ],
    };
    for (final (from, to) in segments) {
      canvas.drawLine(
        centre.translate(from.dx, from.dy),
        centre.translate(to.dx, to.dy),
        paint,
      );
    }
  }

  @override
  bool shouldRepaint(_GlyphPainter oldDelegate) => oldDelegate.kind != kind;
}
