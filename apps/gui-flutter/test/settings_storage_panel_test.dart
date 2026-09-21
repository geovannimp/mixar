import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/settings/settings_storage_panel.dart';

void main() {
  test('formatStorageBytes scales', () {
    expect(formatStorageBytes(BigInt.zero), '0 B');
    expect(formatStorageBytes(BigInt.from(512)), '512 B');
    expect(formatStorageBytes(BigInt.from(2048)), '2.0 KB');
    expect(formatStorageBytes(BigInt.from(3 * 1024 * 1024)), '3.0 MB');
  });

  test('storageShare is proportional', () {
    expect(storageShare(BigInt.zero, BigInt.zero), 0);
    expect(storageShare(BigInt.from(25), BigInt.from(100)), 0.25);
    expect(storageShare(BigInt.from(75), BigInt.from(100)), 0.75);
  });
}
