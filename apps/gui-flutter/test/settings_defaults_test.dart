import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/desktop.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
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

  test('tagWriteBack defaults off and is dirty when changed', () {
    final baseline = defaultAppSettings();
    expect(baseline.tagWriteBack, isFalse);
    expect(
      appSettingsDirty(copyAppSettings(baseline, tagWriteBack: true), baseline),
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

  test('uiLanguage defaults system and dirty when changed', () {
    final baseline = defaultAppSettings();
    expect(baseline.uiLanguage, UiLanguageSetting.system);
    expect(
      appSettingsDirty(
        copyAppSettings(baseline, uiLanguage: UiLanguageSetting.ptBr),
        baseline,
      ),
      isTrue,
    );
    expect(
      appSettingsDirty(
        copyAppSettings(baseline, uiLanguage: UiLanguageSetting.system),
        baseline,
      ),
      isFalse,
    );
  });

  test('vinyl brake stop secs dirty check tolerates the f32 boundary', () {
    // The host reports stop secs as Rust f32 widened to double; the deck panel
    // stepper drafts on the 2-decimal grid. Both display "0.55 s".
    final baseline = copyAppSettings(
      defaultAppSettings(),
      vinylBrakeStopSecs: Float32List.fromList([0.55])[0],
    );
    final draft = copyAppSettings(
      baseline,
      vinylBrakeStopSecs: double.parse(0.55.toStringAsFixed(2)),
    );
    expect(
      baseline.vinylBrakeStopSecs,
      isNot(draft.vinylBrakeStopSecs),
      reason: 'representations must really differ or the tolerance is vacuous',
    );
    expect(appSettingsDirty(draft, baseline), isFalse);
  });

  test('vinyl brake stop secs still flags a real change', () {
    final baseline = copyAppSettings(defaultAppSettings());
    expect(
      appSettingsDirty(
        copyAppSettings(baseline, vinylBrakeStopSecs: 1.25),
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
