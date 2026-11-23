import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/theme/material_theme.dart';
import 'package:gui_flutter/core/theme/mixar_button_style.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/widgets/mixer/deck_key_lock_button.dart';
import 'package:gui_flutter/widgets/mixer/mixer_button.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  Future<void> pumpButton(
    WidgetTester tester, {
    required bool keyLock,
    bool enabled = true,
    ValueChanged<bool>? onChanged,
  }) async {
    final theme = MixarThemeData.dark();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          appSettingsProvider.overrideWith((ref) async => defaultAppSettings()),
        ],
        child: MaterialApp(
          theme: materialUiThemeFromMixar(theme),
          builder: mixarMaterialAppBuilder(theme),
          home: Scaffold(
            body: DeckKeyLockButton(
              keyLabel: 'F#m',
              keyLock: keyLock,
              enabled: enabled,
              onChanged: onChanged ?? (_) {},
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  /// The chip's rendered surface, so a chrome-tree change is handled once.
  BoxDecoration keyLockChrome(WidgetTester tester) {
    final box = tester.widget<DecoratedBox>(
      find
          .descendant(
            of: find.byType(MixerButton),
            matching: find.byType(DecoratedBox),
          )
          .first,
    );
    return box.decoration as BoxDecoration;
  }

  testWidgets('shows lock when key lock on and lockOpen when off', (
    tester,
  ) async {
    await pumpButton(tester, keyLock: true);
    expect(find.text('F#m'), findsOneWidget);
    expect(find.byIcon(LucideIcons.lock), findsOneWidget);
    expect(find.byIcon(LucideIcons.lockOpen), findsNothing);

    await pumpButton(tester, keyLock: false);
    expect(find.byIcon(LucideIcons.lockOpen), findsOneWidget);
    expect(find.byIcon(LucideIcons.lock), findsNothing);
  });

  testWidgets('key lock uses the deck toggle chrome', (tester) async {
    await pumpButton(tester, keyLock: true);

    final button = tester.widget<MixerButton>(
      find.descendant(
        of: find.byType(DeckKeyLockButton),
        matching: find.byType(MixerButton),
      ),
    );
    expect(button.variant, MixarButtonVariant.outline);
    expect(button.selected, isTrue);
  });

  testWidgets('key lock renders the deck toggle chrome', (tester) async {
    final theme = MixarThemeData.dark();

    await pumpButton(tester, keyLock: true);
    final locked = keyLockChrome(tester);
    expect(locked.color, theme.colors.primaryTint);
    expect(locked.borderRadius, theme.style.borderRadius.pill);
    expect(locked.border!.top.color, theme.colors.border);
    expect(locked.border!.top.width, theme.style.borderWidth);
    expect(
      tester.widget<Icon>(find.byIcon(LucideIcons.lock)).color,
      theme.colors.primary,
    );

    await pumpButton(tester, keyLock: false);
    final unlocked = keyLockChrome(tester);
    expect(unlocked.color, theme.colors.card);
    expect(unlocked.borderRadius, theme.style.borderRadius.pill);
    expect(unlocked.border!.top.color, theme.colors.border);
    expect(
      tester.widget<Icon>(find.byIcon(LucideIcons.lockOpen)).color,
      theme.colors.mutedForeground,
    );
  });

  testWidgets('a disabled locked chip does not read as active', (tester) async {
    final theme = MixarThemeData.dark();
    await pumpButton(tester, keyLock: true, enabled: false);

    final chrome = keyLockChrome(tester);
    expect(chrome.color, theme.colors.disable(theme.colors.card));
    expect(
      tester.widget<Icon>(find.byIcon(LucideIcons.lock)).color,
      theme.colors.mutedForeground,
      reason: 'the glyph stays a lock but takes the disabled ink',
    );
  });

  testWidgets('whole button emits absolute desired key lock', (tester) async {
    final desired = <bool>[];
    await pumpButton(tester, keyLock: false, onChanged: desired.add);

    // Tap MixerButton chrome (not Text / Icon children alone).
    await tester.tap(find.byType(MixerButton));
    await tester.pumpAndSettle();
    expect(desired, [true]);

    await pumpButton(tester, keyLock: true, onChanged: desired.add);
    await tester.tap(find.byType(MixerButton));
    await tester.pumpAndSettle();
    expect(desired, [true, false]);
  });

  testWidgets('disabled button does not emit', (tester) async {
    final desired = <bool>[];
    await pumpButton(
      tester,
      keyLock: false,
      enabled: false,
      onChanged: desired.add,
    );

    await tester.tap(find.text('F#m'));
    await tester.pumpAndSettle();
    expect(desired, isEmpty);
  });
}
