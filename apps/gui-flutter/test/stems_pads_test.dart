import 'dart:io';
import 'dart:typed_data';

import 'package:flutter/gestures.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/theme/material_theme.dart';
import 'package:gui_flutter/widgets/mixer/pads/pad_button.dart';
import 'package:gui_flutter/widgets/mixer/pads/stems_pads.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';
import 'support/pad_finders.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  setUpAll(() async {
    Future<void> load(String family, String path) async {
      final loader = FontLoader(family)
        ..addFont(
          Future.value(ByteData.sublistView(File(path).readAsBytesSync())),
        );
      await loader.load();
    }

    await load('Outfit', 'fonts/Outfit-Variable.ttf');
    await load('Space Grotesk', 'fonts/SpaceGrotesk-Variable.ttf');
    await load('Noto Sans Mono', 'fonts/NotoSansMono-Variable.ttf');
  });

  final theme = MixarThemeData.dark();

  Future<void> pumpStems(
    WidgetTester tester, {
    List<bool> mute = const [false, false, false, false],
    int? isolate,
    bool ready = true,
    bool disabled = false,
    double size = 360,
    void Function(int slot)? onPress,
  }) async {
    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromMixar(theme),
        builder: mixarMaterialAppBuilder(theme),
        home: Scaffold(
          body: SizedBox(
            width: size,
            height: size,
            child: StemsPads(
              stemMute: mute,
              stemIsolate: isolate,
              stemsReady: ready,
              stemsGenerating: false,
              disabled: disabled,
              onPress: onPress ?? (_) {},
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  testWidgets('rest pads show every stem with a tinted hairline border', (
    tester,
  ) async {
    await pumpStems(tester);

    expect(find.text('MUTE'), findsNWidgets(4));
    expect(find.text('SOLO'), findsNWidgets(4));
    for (final name in ['Drums', 'Bass', 'Other', 'Vocals']) {
      expect(find.text(name), findsNWidgets(2));
    }

    for (var stem = 0; stem < 4; stem++) {
      final decoration = padDecorationOf(
        tester,
        find.byType(PadButton).at(stem),
      );
      expect(decoration.color, theme.colors.card);
      expect(
        decoration.border!.top.color,
        Color.alphaBlend(
          kStemColors[stem].withValues(alpha: 0.4),
          theme.colors.border,
        ),
      );
    }
  });

  testWidgets('muted and isolated pads fill with the stem color', (
    tester,
  ) async {
    await pumpStems(
      tester,
      mute: const [true, false, false, false],
      isolate: 2,
    );

    final muted = padDecorationOf(tester, find.byType(PadButton).at(0));
    expect(muted.color, kStemColors[0]);
    expect(muted.border!.top.color, kStemColors[0]);

    // Solo pads are slots 4–7; isolate index 2 lights slot 6 (Other).
    final isolated = padDecorationOf(tester, find.byType(PadButton).at(6));
    expect(isolated.color, kStemColors[2]);
    expect(isolated.border!.top.color, kStemColors[2]);

    // A non-lit pad keeps the card fill.
    expect(
      padDecorationOf(tester, find.byType(PadButton).at(1)).color,
      theme.colors.card,
    );
  });

  testWidgets('hover lifts the border to the full stem color', (tester) async {
    await pumpStems(tester);
    final pointer = await tester.createGesture(kind: PointerDeviceKind.mouse);
    addTearDown(pointer.removePointer);
    await pointer.addPointer(location: Offset.zero);

    await pointer.moveTo(tester.getCenter(find.byType(PadButton).at(1)));
    await tester.pumpAndSettle();
    expect(
      padDecorationOf(tester, find.byType(PadButton).at(1)).border!.top.color,
      kStemColors[1],
    );

    await pointer.moveTo(Offset.zero);
    await tester.pumpAndSettle();
    expect(
      padDecorationOf(tester, find.byType(PadButton).at(1)).border!.top.color,
      Color.alphaBlend(
        kStemColors[1].withValues(alpha: 0.4),
        theme.colors.border,
      ),
    );
  });

  testWidgets('pads without stems stay inert and neutral', (tester) async {
    var presses = 0;
    await pumpStems(
      tester,
      mute: const [true, false, false, false],
      ready: false,
      onPress: (_) => presses++,
    );

    final decoration = padDecorationOf(tester, find.byType(PadButton).at(0));
    expect(decoration.color, theme.colors.secondary.withValues(alpha: 0.35));
    expect(decoration.border!.top.color, theme.colors.border);

    // Inert pads must not keep the bright name that made them look enabled.
    final name = tester.widget<Text>(find.text('Drums').first);
    expect(name.style!.color, theme.colors.mutedForeground);

    await tester.tap(find.byType(PadButton).at(0));
    await tester.pumpAndSettle();
    expect(presses, 0);
  });

  testWidgets('pressing a pad reports its slot', (tester) async {
    final pressed = <int>[];
    await pumpStems(tester, onPress: pressed.add);

    await tester.tap(find.byType(PadButton).at(5));
    await tester.pumpAndSettle();
    expect(pressed, [5]);
  });

  testWidgets('header holds action and dot, name centres below it', (
    tester,
  ) async {
    await pumpStems(tester);

    final pad = tester.getRect(find.byType(PadButton).at(0));
    final action = tester.getRect(find.text('MUTE').first);
    final dot = tester.getRect(
      find
          .descendant(
            of: find.byType(PadButton).at(0),
            matching: find.byType(DecoratedBox),
          )
          .last,
    );
    final name = tester.getRect(find.text('Drums').first);

    expect(action.left, closeTo(pad.left + 8, 0.5));
    expect(action.top, closeTo(pad.top + 8, 0.5));
    expect(dot.right, closeTo(pad.right - 8, 0.5));
    expect(dot.center.dy, closeTo(action.center.dy, 0.5));

    expect(name.center.dx, closeTo(pad.center.dx, 0.5));
    // The name box is the region under the header, inset by the pad padding.
    expect(
      name.center.dy,
      closeTo((action.bottom + (pad.bottom - 8)) / 2, 0.5),
    );
  });

  testWidgets('narrow pads scale every stem name identically', (tester) async {
    await pumpStems(tester, size: 240);

    // The widest name sets the shared label slot, so all four render at one
    // scale instead of each pad shrinking only the names that overflow.
    final heights = <double>{
      for (final name in ['Drums', 'Bass', 'Other', 'Vocals'])
        tester.getRect(find.text(name).first).height,
    };
    expect(heights, hasLength(1));
  });
}
