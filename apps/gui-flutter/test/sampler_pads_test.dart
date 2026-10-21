import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/widgets/mixer/pads/sampler_pads.dart';

import 'support/fake_library_transport.dart';

void main() {
  test('FakeLibraryTransport.createSamplerBank returns unique ids', () async {
    final transport = FakeLibraryTransport();
    final first = await transport.createSamplerBank(name: 'One');
    final second = await transport.createSamplerBank(name: 'Two');
    expect(first.id, isNot(second.id));
    expect(second.sortIndex, 1);
    transport.dispose();
  });

  test('SamplerSlot.filled requires path or non-empty label', () {
    expect(const SamplerSlot().filled, isFalse);
    expect(const SamplerSlot(label: '   ').filled, isFalse);
    expect(const SamplerSlot(path: '').filled, isFalse);
    expect(const SamplerSlot(path: 'kick.wav').filled, isTrue);
    expect(const SamplerSlot(label: 'Kick').filled, isTrue);
  });
}
