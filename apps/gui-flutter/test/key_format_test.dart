import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/mixer/key_format.dart';

void main() {
  test('formatDeckKey musical and camelot round-trip', () {
    expect(formatDeckKey(null, KeyDisplayMode.musical), '—');
    expect(formatDeckKey('  ', KeyDisplayMode.camelot), '—');
    expect(formatDeckKey('8A', KeyDisplayMode.musical), 'C#');
    expect(formatDeckKey('Am', KeyDisplayMode.camelot), '1B');
    expect(formatDeckKey('C', KeyDisplayMode.camelot), '1A');
    expect(formatDeckKey('unknown', KeyDisplayMode.camelot), 'unknown');
  });
}
