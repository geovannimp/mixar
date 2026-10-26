import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/theme/material_theme.dart';
import 'package:gui_flutter/core/utils/key_format.dart';
import 'package:gui_flutter/widgets/library/key_pill.dart';
import 'package:gui_flutter/widgets/library/trailing_meta.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  Future<void> pumpMeta(WidgetTester tester, {required double budget}) async {
    final theme = MixarThemeData.light();
    await tester.pumpWidget(
      ProviderScope(
        child: MaterialApp(
          theme: materialUiThemeFromMixar(theme),
          builder: mixarMaterialAppBuilder(theme),
          home: Align(
            alignment: Alignment.topLeft,
            child: SizedBox(
              width: budget,
              child: const TrailingMeta(
                bpm: 128,
                rawKey: '8A',
                keyDisplayMode: KeyDisplayMode.musical,
                keyColorMode: KeyColorMode.off,
                rating: 4,
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pump();
  }

  testWidgets('a lone pill too wide for the budget is hidden', (tester) async {
    await pumpMeta(tester, budget: 20);

    expect(tester.takeException(), isNull);
    expect(find.byKey(kTrackRatingPillKey), findsNothing);
    expect(find.text('128.0 BPM'), findsNothing);
  });

  testWidgets('a budget that seats one pill keeps it and drops the rest', (
    tester,
  ) async {
    // The rating takes its 40px slot, leaving 4px: less than the 6px gap before
    // the next pill, so the BPM and key labels are left out.
    await pumpMeta(tester, budget: 44);

    expect(tester.takeException(), isNull);
    expect(find.byKey(kTrackRatingPillKey), findsOneWidget);
    expect(find.text('128.0 BPM'), findsNothing);
    expect(find.text('Am'), findsNothing);
  });

  testWidgets('a budget with room keeps the next pill too', (tester) async {
    await pumpMeta(tester, budget: 400);

    expect(tester.takeException(), isNull);
    expect(find.byKey(kTrackRatingPillKey), findsOneWidget);
    expect(find.text('128.0 BPM'), findsOneWidget);
    expect(find.text('Am'), findsOneWidget);
    expect(find.byType(KeyPill), findsOneWidget);
  });
}
