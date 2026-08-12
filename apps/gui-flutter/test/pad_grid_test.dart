import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/mixer/pads/pad_grid.dart';

void main() {
  test('PadGrid rejects wrong child counts', () {
    expect(
      () => PadGrid(children: const [SizedBox(), SizedBox()]),
      throwsArgumentError,
    );
    expect(
      () => PadGrid(
        children: List<Widget>.generate(8, (_) => const SizedBox()),
      ),
      returnsNormally,
    );
  });
}
