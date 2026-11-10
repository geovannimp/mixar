import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/widgets/settings/settings_section_card.dart';
import 'package:gui_flutter/widgets/settings/settings_section_header.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  testWidgets('renders title, subtitle, and child', (tester) async {
    await tester.pumpWidget(
      mixarTestMaterialApp(
        theme: MixarThemeData.dark(),
        home: const Scaffold(
          body: SettingsSectionCard(
            title: 'Engine',
            subtitle: 'Backend and buffer.',
            child: Text('field-body'),
          ),
        ),
      ),
    );
    expect(find.text('Engine'), findsOneWidget);
    expect(find.text('Backend and buffer.'), findsOneWidget);
    expect(find.text('field-body'), findsOneWidget);
    expect(find.byType(SettingsSectionHeader), findsOneWidget);
    expect(find.byType(Row), findsNothing);
  }, semanticsEnabled: false);

  testWidgets('rules off the header from its fields', (tester) async {
    Future<void> pump({Widget? child, bool decorated = true}) =>
        tester.pumpWidget(
          mixarTestMaterialApp(
            theme: MixarThemeData.dark(),
            home: Scaffold(
              body: SettingsSectionCard(
                title: 'Theme',
                subtitle:
                    'Brightness and the accent color used by brand chrome.',
                decorated: decorated,
                child: child,
              ),
            ),
          ),
        );

    await pump(child: const Text('field-body'));
    final card = tester.getRect(find.byType(SettingsSectionCard));
    final title = tester.getRect(find.text('Theme'));
    final rule = tester.getRect(find.byKey(settingsCardRuleKey));
    final body = tester.getRect(find.text('field-body'));

    // Full-bleed, like the card's own hairlines, and between the two blocks.
    expect(rule.width, card.width);
    expect(rule.top, greaterThan(title.bottom));
    expect(body.top, greaterThan(rule.bottom));

    // Header-only cards (toggle off, nothing to configure) have no body and
    // must not show a stray line once the fields have collapsed away.
    await pump();
    await tester.pumpAndSettle();
    expect(find.byKey(settingsCardRuleKey), findsNothing);

    // The setup stage turns the band off; the rule is band chrome too.
    await pump(child: const Text('field-body'), decorated: false);
    expect(find.byKey(settingsCardRuleKey), findsNothing);
    expect(find.text('field-body'), findsOneWidget);
  }, semanticsEnabled: false);

  testWidgets(
    'lines a narrow body up with the header instead of centering it',
    (tester) async {
      await tester.pumpWidget(
        mixarTestMaterialApp(
          theme: MixarThemeData.dark(),
          home: const Scaffold(
            body: SettingsSectionCard(
              title: 'Import from Rekordbox',
              subtitle: 'Bring your Rekordbox library into Mixar.',
              child: Text('No Rekordbox library found on this computer.'),
            ),
          ),
        ),
      );

      final titleLeft = tester
          .getTopLeft(find.text('Import from Rekordbox'))
          .dx;
      final bodyLeft = tester
          .getTopLeft(find.text('No Rekordbox library found on this computer.'))
          .dx;
      final bodyRight = tester
          .getBottomRight(
            find.text('No Rekordbox library found on this computer.'),
          )
          .dx;
      final cardLeft = tester.getTopLeft(find.byType(SettingsSectionCard)).dx;
      final cardRight = tester
          .getBottomRight(find.byType(SettingsSectionCard))
          .dx;

      expect(bodyLeft, titleLeft, reason: 'body starts where the title starts');
      expect(
        bodyLeft - cardLeft,
        lessThan(cardRight - bodyRight),
        reason: 'the slack sits after the text, not split around it',
      );
    },
    semanticsEnabled: false,
  );

  testWidgets('title-only card renders no description line', (tester) async {
    await tester.pumpWidget(
      mixarTestMaterialApp(
        theme: MixarThemeData.dark(),
        home: const Scaffold(
          body: SettingsSectionCard(
            title: 'Visible fields',
            child: Text('field-body'),
          ),
        ),
      ),
    );
    expect(find.text('Visible fields'), findsOneWidget);
    expect(find.text('field-body'), findsOneWidget);
    expect(
      find.descendant(
        of: find.byType(SettingsSectionHeader),
        matching: find.byType(Text),
      ),
      findsOneWidget,
      reason: 'a card without a subtitle must not paint an empty line',
    );
  }, semanticsEnabled: false);

  testWidgets('places trailing beside the header', (tester) async {
    await tester.pumpWidget(
      mixarTestMaterialApp(
        theme: MixarThemeData.dark(),
        home: const Scaffold(
          body: SettingsSectionCard(
            title: 'Mappings',
            subtitle: 'Stored maps.',
            trailing: Text('Update All'),
            child: Text('list'),
          ),
        ),
      ),
    );
    expect(find.text('Update All'), findsOneWidget);
    expect(find.text('list'), findsOneWidget);
    expect(find.byType(Row), findsOneWidget);
  }, semanticsEnabled: false);

  testWidgets('omits body spacing when child is null', (tester) async {
    await tester.pumpWidget(
      mixarTestMaterialApp(
        theme: MixarThemeData.dark(),
        home: const Scaffold(
          body: SettingsSectionCard(
            title: 'Preview bus',
            subtitle: 'Headphones / cue.',
            trailing: Text('toggle'),
          ),
        ),
      ),
    );
    expect(find.text('Preview bus'), findsOneWidget);
    expect(find.text('toggle'), findsOneWidget);
    expect(find.text('field-body'), findsNothing);
  }, semanticsEnabled: false);

  testWidgets('animates the body open and closed', (tester) async {
    const body = Text('field-body');
    Widget card(Widget? child) => mixarTestMaterialApp(
      theme: MixarThemeData.dark(),
      home: Scaffold(
        body: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            SettingsSectionCard(
              title: 'Preview bus',
              subtitle: 'Headphones / cue.',
              trailing: const Text('toggle'),
              child: child,
            ),
          ],
        ),
      ),
    );
    double cardHeight() =>
        tester.getSize(find.byType(SettingsSectionCard)).height;

    await tester.pumpWidget(card(null));
    final closedHeight = cardHeight();

    // Opening grows the card over several frames instead of snapping.
    await tester.pumpWidget(card(body));
    await tester.pump(const Duration(milliseconds: 100));
    final openingHeight = cardHeight();
    expect(openingHeight, greaterThan(closedHeight));
    expect(find.text('field-body'), findsOneWidget);
    await tester.pumpAndSettle();
    final openHeight = cardHeight();
    expect(openHeight, greaterThan(openingHeight));

    // Closing keeps the fields mounted while the card shrinks, then removes
    // them once the transition lands.
    await tester.pumpWidget(card(null));
    await tester.pump(const Duration(milliseconds: 100));
    expect(find.text('field-body'), findsOneWidget);
    expect(cardHeight(), greaterThan(closedHeight));
    expect(cardHeight(), lessThan(openHeight));
    await tester.pumpAndSettle();
    expect(find.text('field-body'), findsNothing);
    expect(cardHeight(), closedHeight);
  }, semanticsEnabled: false);

  testWidgets('scrolls the page so a newly opened body is visible', (
    tester,
  ) async {
    final scroll = ScrollController();
    addTearDown(scroll.dispose);
    const bodyKey = ValueKey('body');
    Widget page(Widget? body) => mixarTestMaterialApp(
      theme: MixarThemeData.dark(),
      home: Scaffold(
        body: SingleChildScrollView(
          controller: scroll,
          child: Column(
            children: [
              const SizedBox(height: 400),
              SettingsSectionCard(
                title: 'Preview bus',
                subtitle: 'Headphones / cue.',
                trailing: const Text('toggle'),
                child: body,
              ),
              const SizedBox(height: 100),
            ],
          ),
        ),
      ),
    );

    await tester.pumpWidget(page(null));
    scroll.jumpTo(scroll.position.maxScrollExtent);
    await tester.pump();
    final collapsedOffset = scroll.position.pixels;

    await tester.pumpWidget(
      page(const SizedBox(key: bodyKey, height: 300, width: double.infinity)),
    );
    await tester.pumpAndSettle();

    expect(
      scroll.position.pixels,
      greaterThan(collapsedOffset),
      reason: 'the page must scroll to reach the opened fields',
    );
    final body = tester.getRect(find.byKey(bodyKey));
    expect(
      body.bottom,
      lessThanOrEqualTo(584),
      reason: 'opened fields clear the viewport edge by the card inset',
    );
  }, semanticsEnabled: false);

  testWidgets('does not scroll for a body that starts open', (tester) async {
    final scroll = ScrollController();
    addTearDown(scroll.dispose);
    await tester.pumpWidget(
      mixarTestMaterialApp(
        theme: MixarThemeData.dark(),
        home: Scaffold(
          body: SingleChildScrollView(
            controller: scroll,
            child: const Column(
              children: [
                SizedBox(height: 400),
                SettingsSectionCard(
                  title: 'Preview bus',
                  trailing: Text('toggle'),
                  child: SizedBox(height: 300, width: double.infinity),
                ),
              ],
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(scroll.position.pixels, 0);
  }, semanticsEnabled: false);

  testWidgets('reveals the end of a body taller than the viewport', (
    tester,
  ) async {
    final scroll = ScrollController();
    addTearDown(scroll.dispose);
    const bodyKey = ValueKey('body');
    Widget page(Widget? body) => mixarTestMaterialApp(
      theme: MixarThemeData.dark(),
      home: Scaffold(
        body: SingleChildScrollView(
          controller: scroll,
          child: Column(
            children: [
              const SizedBox(height: 400),
              SettingsSectionCard(
                title: 'Preview bus',
                trailing: const Text('toggle'),
                child: body,
              ),
              const SizedBox(height: 100),
            ],
          ),
        ),
      ),
    );

    await tester.pumpWidget(page(null));
    scroll.jumpTo(scroll.position.maxScrollExtent);
    await tester.pump();
    await tester.pumpWidget(
      page(const SizedBox(key: bodyKey, height: 800, width: double.infinity)),
    );
    await tester.pumpAndSettle();

    final body = tester.getRect(find.byKey(bodyKey));
    expect(
      body.bottom,
      lessThanOrEqualTo(600),
      reason: 'the last fields must be visible even when the body overflows',
    );
    expect(
      body.top,
      lessThan(0),
      reason: 'a body taller than the viewport trades its top for its end',
    );
  }, semanticsEnabled: false);

  testWidgets('re-reveals when late content grows the opened body', (
    tester,
  ) async {
    final scroll = ScrollController();
    addTearDown(scroll.dispose);
    const bodyKey = ValueKey('body');
    Widget page(double? parentHeight) => mixarTestMaterialApp(
      theme: MixarThemeData.dark(),
      home: Scaffold(
        body: SingleChildScrollView(
          controller: scroll,
          child: Column(
            children: [
              const SizedBox(height: 400),
              SettingsSectionCard(
                title: 'Preview bus',
                trailing: const Text('toggle'),
                child: parentHeight == null
                    ? null
                    : SizedBox(
                        key: bodyKey,
                        height: parentHeight,
                        width: double.infinity,
                      ),
              ),
              const SizedBox(height: 100),
            ],
          ),
        ),
      ),
    );

    await tester.pumpWidget(page(null));
    await tester.pumpWidget(page(200));
    await tester.pumpAndSettle();
    expect(tester.getRect(find.byKey(bodyKey)).bottom, lessThanOrEqualTo(600));

    // Device lists land after the card opened and push the fields down.
    await tester.pumpWidget(page(500));
    await tester.pumpAndSettle();
    expect(
      tester.getRect(find.byKey(bodyKey)).bottom,
      lessThanOrEqualTo(584),
      reason: 'late growth must not leave the fields cropped again',
    );
  }, semanticsEnabled: false);

  testWidgets('scrolling fields open with reduced motion', (tester) async {
    final scroll = ScrollController();
    addTearDown(scroll.dispose);
    const bodyKey = ValueKey('body');
    Widget page(Widget? body) => mixarTestMaterialApp(
      theme: MixarThemeData.dark(),
      wrapChild: (child) => Builder(
        builder: (context) => MediaQuery(
          data: MediaQuery.of(context).copyWith(disableAnimations: true),
          child: child,
        ),
      ),
      home: Scaffold(
        body: SingleChildScrollView(
          controller: scroll,
          child: Column(
            children: [
              const SizedBox(height: 400),
              SettingsSectionCard(
                title: 'Preview bus',
                trailing: const Text('toggle'),
                child: body,
              ),
              const SizedBox(height: 100),
            ],
          ),
        ),
      ),
    );

    await tester.pumpWidget(page(null));
    scroll.jumpTo(scroll.position.maxScrollExtent);
    await tester.pump();
    final collapsedOffset = scroll.position.pixels;

    await tester.pumpWidget(
      page(const SizedBox(key: bodyKey, height: 300, width: double.infinity)),
    );
    await tester.pumpAndSettle();

    expect(tester.takeException(), isNull);
    expect(scroll.position.pixels, greaterThan(collapsedOffset));
    expect(tester.getRect(find.byKey(bodyKey)).bottom, lessThanOrEqualTo(584));
  }, semanticsEnabled: false);

  testWidgets('collapsing fields leave the semantics tree at once', (
    tester,
  ) async {
    Widget card(Widget? child) => mixarTestMaterialApp(
      theme: MixarThemeData.dark(),
      home: Scaffold(
        body: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            SettingsSectionCard(
              title: 'Preview bus',
              trailing: const Text('toggle'),
              child: child,
            ),
          ],
        ),
      ),
    );

    await tester.pumpWidget(card(const Text('field-body')));
    expect(find.semantics.byLabel('field-body'), findsOne);

    await tester.pumpWidget(card(null));
    await tester.pump(const Duration(milliseconds: 100));
    expect(
      find.text('field-body'),
      findsOneWidget,
      reason: 'the body is still fading out',
    );
    expect(
      find.semantics.byLabel('field-body'),
      findsNothing,
      reason: 'a collapsing body must not stay readable to assistive tech',
    );
    await tester.pumpAndSettle();
    expect(find.text('field-body'), findsNothing);
  });

  testWidgets('rebuilding an open card does not scroll the page', (
    tester,
  ) async {
    final scroll = ScrollController();
    addTearDown(scroll.dispose);
    Widget page(String subtitle) => mixarTestMaterialApp(
      theme: MixarThemeData.dark(),
      home: Scaffold(
        body: SingleChildScrollView(
          controller: scroll,
          child: Column(
            children: [
              const SizedBox(height: 400),
              SettingsSectionCard(
                title: 'Preview bus',
                subtitle: subtitle,
                trailing: const Text('toggle'),
                child: const SizedBox(height: 300, width: double.infinity),
              ),
              const SizedBox(height: 100),
            ],
          ),
        ),
      ),
    );

    await tester.pumpWidget(page('Headphones / cue.'));
    await tester.pumpAndSettle();
    expect(scroll.position.pixels, 0);

    // Provider/settings updates rebuild cards long after they opened.
    await tester.pumpWidget(page('Headphones / cue. (updated)'));
    await tester.pumpAndSettle();
    expect(scroll.position.pixels, 0);
  }, semanticsEnabled: false);

  testWidgets('leaves the page alone when the opened body already fits', (
    tester,
  ) async {
    final scroll = ScrollController();
    addTearDown(scroll.dispose);
    Widget page(Widget? body) => mixarTestMaterialApp(
      theme: MixarThemeData.dark(),
      home: Scaffold(
        body: SingleChildScrollView(
          controller: scroll,
          child: Column(
            children: [
              SettingsSectionCard(
                title: 'Preview bus',
                trailing: const Text('toggle'),
                child: body,
              ),
              const SizedBox(height: 100),
            ],
          ),
        ),
      ),
    );

    await tester.pumpWidget(page(null));
    await tester.pumpWidget(
      page(const SizedBox(height: 200, width: double.infinity)),
    );
    await tester.pumpAndSettle();
    expect(scroll.position.pixels, 0);
  }, semanticsEnabled: false);

  testWidgets('undecorated mode keeps the content but drops the band chrome', (
    tester,
  ) async {
    Future<void> pump({required bool decorated}) => tester.pumpWidget(
      mixarTestMaterialApp(
        theme: MixarThemeData.dark(),
        home: Scaffold(
          body: SettingsSectionCard(
            title: 'Import from Mixxx',
            decorated: decorated,
            child: const Text('body'),
          ),
        ),
      ),
    );

    await pump(decorated: true);
    final decoratedLeft = tester.getTopLeft(find.text('Import from Mixxx')).dx;

    await pump(decorated: false);
    final bareLeft = tester.getTopLeft(find.text('Import from Mixxx')).dx;

    // No card fill and no hairlines: only the header and body remain.
    expect(
      find.descendant(
        of: find.byType(SettingsSectionCard),
        matching: find.byType(ColoredBox),
      ),
      findsNothing,
    );
    // The band's inner inset is gone, so the content starts further left.
    expect(bareLeft, lessThan(decoratedLeft));
    expect(find.text('body'), findsOneWidget);
  }, semanticsEnabled: false);
}
