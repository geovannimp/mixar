import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';

void main() {
  test('equal collection contents are not dirty', () {
    final baseline = defaultAppSettings();
    final restored = copyAppSettings(
      baseline,
      libraryTableColumns: List<String>.from(baseline.libraryTableColumns),
      deckDefaultSamplerBankId: List<String?>.from(
        baseline.deckDefaultSamplerBankId,
      ),
      tempoRangeSteps: Float32List.fromList(baseline.tempoRangeSteps),
    );
    expect(appSettingsDirty(baseline, restored), isFalse);
    expect(
      identical(baseline.libraryTableColumns, restored.libraryTableColumns),
      isFalse,
    );
  });

  test('library column edits are dirty', () {
    final baseline = defaultAppSettings();
    final next = List<String>.from(baseline.libraryTableColumns)
      ..remove('artist');
    expect(
      appSettingsDirty(
        copyAppSettings(baseline, libraryTableColumns: next),
        baseline,
      ),
      isTrue,
    );
  });

  test('trusted controller edits are dirty', () {
    final baseline = defaultAppSettings();
    expect(
      appSettingsDirty(
        copyAppSettings(
          baseline,
          trustedControllerDeviceIds: ['pioneer.ddj-400'],
        ),
        baseline,
      ),
      isTrue,
    );
  });

  test('showTooltips edits are dirty', () {
    final baseline = defaultAppSettings();
    expect(baseline.showTooltips, isTrue);
    expect(
      appSettingsDirty(
        copyAppSettings(baseline, showTooltips: false),
        baseline,
      ),
      isTrue,
    );
  });
}
