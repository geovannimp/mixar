import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/theme/material_theme.dart';
import 'package:gui_flutter/widgets/shell/mixar_input.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';
import 'package:shadcn_ui/shadcn_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  late MixarThemeData theme;

  Future<void> pumpInput(WidgetTester tester, MixarInput input) async {
    theme = MixarThemeData.dark();
    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromMixar(theme),
        builder: mixarMaterialAppBuilder(theme),
        home: Scaffold(body: Center(child: input)),
      ),
    );
  }

  /// The decoration handed to [ShadInput]; `null` means the theme's own input
  /// decoration is left alone.
  ShadDecoration? decorationOf(WidgetTester tester) =>
      tester.widget<ShadInput>(find.byType(ShadInput)).decoration;

  /// Effective label colour — [MixarInput] merges its style through
  /// `DefaultTextStyle` rather than onto the [Text] itself.
  Color? labelColor(WidgetTester tester, String label) =>
      DefaultTextStyle.of(tester.element(find.text(label))).style.color;

  testWidgets(
    'errorText renders the message and paints the label destructive',
    (tester) async {
      await pumpInput(
        tester,
        MixarInput(
          label: const Text('BPM'),
          errorText: 'BPM must be a positive number',
        ),
      );

      expect(find.text('BPM must be a positive number'), findsOneWidget);
      expect(
        labelColor(tester, 'BPM'),
        theme.colors.destructive,
        reason: 'label reads as invalid',
      );

      final decoration = decorationOf(tester);
      expect(decoration, isNotNull, reason: 'only the error flag is supplied');
      expect(
        decoration!.hasError,
        isTrue,
        reason:
            'hasError is what makes ShadInput paint its error border '
            '(ShadInput merges the theme border around it in its own build)',
      );
    },
  );

  testWidgets('borderless keeps its flat border while showing an error', (
    tester,
  ) async {
    await pumpInput(
      tester,
      const MixarInput(
        label: Text('Query'),
        borderless: true,
        errorText: 'Nope',
      ),
    );

    // The message still reads even though the field stays flat.
    expect(find.text('Nope'), findsOneWidget);

    final decoration = decorationOf(tester);
    expect(decoration, isNotNull);
    expect(decoration!.hasError, isTrue);
    expect(
      identical(decoration.border, ShadBorder.none),
      isTrue,
      reason: 'ShadDecoration.none wins the merge, so no border is painted',
    );
  });

  testWidgets('a label-less field still reports its error text', (
    tester,
  ) async {
    await pumpInput(
      tester,
      const MixarInput(borderless: true, errorText: 'Bad query'),
    );

    // No label, but the message is still rendered (and no empty row when it is
    // cleared).
    expect(find.text('Bad query'), findsOneWidget);
    expect(decorationOf(tester)!.hasError, isTrue);
  });

  testWidgets('an empty errorText is not an error state', (tester) async {
    await pumpInput(
      tester,
      const MixarInput(label: Text('Query'), errorText: ''),
    );

    expect(
      decorationOf(tester),
      isNull,
      reason: 'empty message counts as none',
    );
  });

  testWidgets('no error leaves a normal field on the theme decoration', (
    tester,
  ) async {
    await pumpInput(tester, const MixarInput(label: Text('Title')));

    expect(
      decorationOf(tester),
      isNull,
      reason: 'theme input decoration is untouched when there is no error',
    );
    expect(find.text('Title'), findsOneWidget);
  });
}
