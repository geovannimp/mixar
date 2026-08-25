import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/mixer/key_format.dart';

void main() {
  test('formatDeckKey musical and camelot Mixed In Key round-trip', () {
    expect(formatDeckKey(null, KeyDisplayMode.musical), '—');
    expect(formatDeckKey('  ', KeyDisplayMode.camelot), '—');
    expect(formatDeckKey('C', KeyDisplayMode.camelot), '8B');
    expect(formatDeckKey('Am', KeyDisplayMode.camelot), '8A');
    expect(formatDeckKey('8B', KeyDisplayMode.musical), 'C');
    expect(formatDeckKey('8A', KeyDisplayMode.musical), 'Am');
    expect(formatDeckKey('1A', KeyDisplayMode.musical), 'G#m');
    expect(formatDeckKey('unknown', KeyDisplayMode.camelot), 'unknown');
  });
}
