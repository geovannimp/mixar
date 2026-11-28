import 'dart:typed_data';

import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

/// Renders the [RepaintBoundary] behind [key] as raw RGBA rows.
///
/// The image is disposed once its bytes are read, so a test can capture
/// repeatedly without retaining native image resources.
Future<Uint8List> captureRgba(
  WidgetTester tester,
  GlobalKey key, {
  double pixelRatio = 1,
}) async {
  final boundary =
      key.currentContext!.findRenderObject()! as RenderRepaintBoundary;
  final bytes = await tester.runAsync(() async {
    final image = await boundary.toImage(pixelRatio: pixelRatio);
    try {
      final data = await image.toByteData();
      return data!.buffer.asUint8List();
    } finally {
      image.dispose();
    }
  });
  return bytes!;
}
