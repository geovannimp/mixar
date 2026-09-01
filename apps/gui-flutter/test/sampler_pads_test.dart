import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/mixer/pads/sampler_pads.dart';

void main() {
  test('SamplerSlot.filled requires path or non-empty label', () {
    expect(const SamplerSlot().filled, isFalse);
    expect(const SamplerSlot(label: '   ').filled, isFalse);
    expect(const SamplerSlot(path: '').filled, isFalse);
    expect(const SamplerSlot(path: 'kick.wav').filled, isTrue);
    expect(const SamplerSlot(label: 'Kick').filled, isTrue);
  });

  test('cycleSamplerBankIndex wraps and falls back to 0', () {
    expect(cycleSamplerBankIndex(activeIndex: 0, direction: 1, length: 2), 1);
    expect(cycleSamplerBankIndex(activeIndex: 1, direction: 1, length: 2), 0);
    expect(cycleSamplerBankIndex(activeIndex: 0, direction: -1, length: 2), 1);
    expect(cycleSamplerBankIndex(activeIndex: -1, direction: 1, length: 3), 1);
    expect(cycleSamplerBankIndex(activeIndex: 0, direction: 1, length: 0), -1);
  });
}
