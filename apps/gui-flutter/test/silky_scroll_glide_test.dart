import 'package:flutter/gestures.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/silky_scroll_glide.dart';
import 'package:material_ui/material_ui.dart';
import 'package:silky_scroll/silky_scroll.dart';

void main() {
  testWidgets('cancelSilkyGlide stops a glide started through a preset', (
    tester,
  ) async {
    final controller = ScrollController();
    addTearDown(controller.dispose);
    final scrollKey = GlobalKey();

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SizedBox(
            width: 300,
            height: 200,
            child: SilkySingleChildScrollView(
              key: scrollKey,
              controller: controller,
              physics: const ClampingScrollPhysics(),
              child: Column(
                children: [
                  for (var i = 0; i < 50; i++)
                    SizedBox(height: 40, child: Text('row $i')),
                ],
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    await tester.sendEventToBinding(
      PointerScrollEvent(
        position: tester.getCenter(find.byType(SilkySingleChildScrollView)),
        scrollDelta: const Offset(0, 300),
      ),
    );
    await tester.pump();
    final beforeCancel = controller.offset;
    expect(beforeCancel, greaterThan(0));

    cancelSilkyGlide(scrollKey);
    // Sample several short frames so an uncancelled ticker keeps moving.
    for (var i = 0; i < 20; i++) {
      await tester.pump(const Duration(milliseconds: 16));
    }

    expect(controller.offset, closeTo(beforeCancel, 0.01));
  });

  testWidgets('a press hands a wheel glide off to the scrollable', (
    tester,
  ) async {
    final controller = ScrollController();
    addTearDown(controller.dispose);
    final scrollKey = GlobalKey();

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SizedBox(
            width: 300,
            height: 200,
            child: SilkyGlideHandoff(
              scrollKey: scrollKey,
              child: SilkySingleChildScrollView(
                key: scrollKey,
                controller: controller,
                physics: const ClampingScrollPhysics(),
                child: Column(
                  children: [
                    for (var i = 0; i < 50; i++)
                      SizedBox(height: 40, child: Text('row $i')),
                  ],
                ),
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    await tester.sendEventToBinding(
      PointerScrollEvent(
        position: tester.getCenter(find.byType(SilkySingleChildScrollView)),
        scrollDelta: const Offset(0, 300),
      ),
    );
    await tester.pump();
    expect(controller.offset, greaterThan(0));

    final gesture = await tester.startGesture(
      tester.getCenter(find.byType(SilkySingleChildScrollView)),
    );
    await tester.pump(const Duration(milliseconds: 16));
    final pressed = controller.offset;
    for (var i = 0; i < 20; i++) {
      await tester.pump(const Duration(milliseconds: 16));
    }

    expect(controller.offset, closeTo(pressed, 0.01));
    await gesture.up();
  });
}
