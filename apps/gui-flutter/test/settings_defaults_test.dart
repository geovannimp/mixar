import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/shell/desktop.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';

void main() {
  test('equal collection contents are not dirty', () {
    final baseline = defaultAppSettings();
    final restored = copyAppSettings(
      baseline,
      deckDefaultSamplerBankId: List<String?>.from(
        baseline.deckDefaultSamplerBankId,
      ),
      tempoRangeSteps: Float32List.fromList(baseline.tempoRangeSteps),
    );
    expect(appSettingsDirty(baseline, restored), isFalse);
  });

  test('row density defaults comfortable and is dirty when changed', () {
    final baseline = defaultAppSettings();
    expect(baseline.libraryRowDensity, LibraryRowDensitySetting.comfortable);
    expect(
      appSettingsDirty(
        copyAppSettings(
          baseline,
          libraryRowDensity: LibraryRowDensitySetting.compact,
        ),
        baseline,
      ),
      isTrue,
    );
  });

  test('normalizeAppSettings keeps the saved row density', () {
    final compact = copyAppSettings(
      defaultAppSettings(),
      libraryRowDensity: LibraryRowDensitySetting.compact,
    );
    expect(
      normalizeAppSettings(compact).libraryRowDensity,
      LibraryRowDensitySetting.compact,
    );
  });

  test('row density maps to the layout the list renders', () {
    expect(
      libraryRowDensityFromSettings(LibraryRowDensitySetting.comfortable),
      LibraryRowDensity.comfortable,
    );
    expect(
      libraryRowDensityFromSettings(LibraryRowDensitySetting.compact),
      LibraryRowDensity.compact,
    );
    // The toggle flips between the two, and only between the two.
    expect(LibraryRowDensity.compact.other, LibraryRowDensity.comfortable);
    expect(LibraryRowDensity.comfortable.other, LibraryRowDensity.compact);
    expect(
      LibraryRowDensity.values.map((d) => d.other),
      containsAll(LibraryRowDensity.values),
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

  test('stemsFormat defaults opus and dirty when changed', () {
    final baseline = defaultAppSettings();
    expect(baseline.stemsFormat, 'opus');
    expect(
      appSettingsDirty(
        copyAppSettings(baseline, stemsFormat: 'flac'),
        baseline,
      ),
      isTrue,
    );
  });

  test('selectStyle defaults auto and is dirty when changed', () {
    final baseline = defaultAppSettings();
    expect(baseline.selectStyle, SelectStyleSetting.auto);
    expect(
      appSettingsDirty(
        copyAppSettings(baseline, selectStyle: SelectStyleSetting.mobile),
        baseline,
      ),
      isTrue,
    );
  });

  test('effectiveSelectStyle maps auto from isDesktopWindow', () {
    addTearDown(() => debugOverrideDesktopWindow = null);

    debugOverrideDesktopWindow = true;
    expect(
      effectiveSelectStyle(SelectStyleSetting.auto),
      SelectStyleSetting.desktop,
    );
    expect(
      effectiveSelectStyle(SelectStyleSetting.mobile),
      SelectStyleSetting.mobile,
    );

    debugOverrideDesktopWindow = false;
    expect(
      effectiveSelectStyle(SelectStyleSetting.auto),
      SelectStyleSetting.mobile,
    );
    expect(
      effectiveSelectStyle(SelectStyleSetting.desktop),
      SelectStyleSetting.desktop,
    );
  });
}
