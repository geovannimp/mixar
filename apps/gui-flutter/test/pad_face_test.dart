import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/widgets/mixer/pads/pad_face.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

import 'support/mixar_material_app.dart';

/// The caption pad squares carry ("SAMPLE", "HOT CUE", …).
const _caption = 'HOT CUE';

Future<void> _pumpFace(WidgetTester tester, Size size) async {
  final theme = MixarThemeData.dark();
  await tester.pumpWidget(
    mixarTestMaterialApp(
      theme: theme,
      home: Center(
        child: SizedBox.fromSize(
          size: size,
          child: const PadFace(label: Text('1'), action: _caption),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

void main() {
  testWidgets('a pad too narrow for the caption keeps its body only', (
    tester,
  ) async {
    await _pumpFace(tester, const Size(12, 12));

    expect(find.text(_caption), findsNothing);
    expect(find.text('1'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('a normal pad keeps the caption over its body', (tester) async {
    await _pumpFace(tester, const Size(60, 60));

    expect(find.text(_caption), findsOneWidget);
    expect(find.text('1'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });
}
