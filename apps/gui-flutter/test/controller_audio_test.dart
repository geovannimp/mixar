import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/controller_audio.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/src/rust/api/controller.dart';
import 'package:gui_flutter/src/rust/api/engine.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';

OutputDevice _device(
  String id,
  String name, {
  int maxChannels = 4,
  bool isDefault = false,
}) => OutputDevice(
  id: id,
  name: name,
  isDefault: isDefault,
  maxChannels: maxChannels,
  defaultSampleRates: Uint32List(0),
  sampleRateRanges: const [],
);

const _hints = ControllerAudioHints(
  outputNameContains: ['DDJ-400'],
  masterChannels: ControllerChannelPair(left: 1, right: 2),
  headphoneChannels: ControllerChannelPair(left: 3, right: 4),
);

const _mappingId = 'ddj-400';
const _controllerLabel = 'DDJ-400';

AppSettings? _apply(
  AppSettings settings,
  ControllerAudioHints hints,
  OutputDevice device,
) => applyControllerAudioOutputs(
  settings,
  hints,
  device,
  mappingId: _mappingId,
  controllerLabel: _controllerLabel,
);

void main() {
  group('matchControllerAudioOutput', () {
    test('matches device names case-insensitively by substring', () {
      final device = matchControllerAudioOutput([
        _device('usb', 'USB Audio Device'),
        _device('ddj', 'Pioneer DJ DDJ-400'),
      ], _hints);
      expect(device?.id, 'ddj');
    });

    test('an exact name beats a partial match', () {
      final device = matchControllerAudioOutput([
        _device('partial', 'Pioneer DDJ-400 Audio'),
        _device('exact', 'DDJ-400'),
      ], _hints);
      expect(device?.id, 'exact');
    });

    test('an exact name beats a partial match that is the system default', () {
      final device = matchControllerAudioOutput([
        _device('partial', 'Pioneer DDJ-400 Audio', isDefault: true),
        _device('exact', 'DDJ-400'),
      ], _hints);
      expect(device?.id, 'exact');
    });

    test('skips devices without enough channels for the routes', () {
      final device = matchControllerAudioOutput([
        _device('stereo', 'DDJ-400', maxChannels: 2),
      ], _hints);
      expect(device, isNull);
    });

    test('ignores the synthetic default entry and empty hints', () {
      expect(
        matchControllerAudioOutput([_device('default', 'DDJ-400')], _hints),
        isNull,
      );
      expect(
        matchControllerAudioOutput([
          _device('ddj', 'DDJ-400'),
        ], const ControllerAudioHints(outputNameContains: ['  '])),
        isNull,
      );
    });
  });

  group('applyControllerAudioOutputs', () {
    test('routes master and headphone buses and enables preview', () {
      final device = _device('ddj', 'DDJ-400');
      final updated = _apply(defaultAppSettings(), _hints, device);
      expect(updated, isNotNull);
      expect(updated!.masterBus.deviceId, 'ddj');
      expect(updated.masterBus.leftChannel, 1);
      expect(updated.masterBus.rightChannel, 2);
      expect(updated.previewEnabled, isTrue);
      expect(updated.previewBus.deviceId, 'ddj');
      expect(updated.previewBus.leftChannel, 3);
      expect(updated.previewBus.rightChannel, 4);
    });

    test('uses a declared sample rate for both buses', () {
      final device = _device('ddj', 'DDJ-400');
      final updated = _apply(
        defaultAppSettings(),
        const ControllerAudioHints(
          outputNameContains: ['DDJ-400'],
          masterChannels: ControllerChannelPair(left: 1, right: 2),
          sampleRate: 48000,
        ),
        device,
      );
      expect(updated?.masterBus.sampleRate, 48000);
      expect(updated?.previewEnabled, isFalse);
    });

    test('does nothing when the master bus already uses the device', () {
      final device = _device('ddj', 'DDJ-400');
      final settings = copyAppSettings(
        defaultAppSettings(),
        masterBus: const BusRouteSettings(
          deviceId: 'ddj',
          leftChannel: 1,
          rightChannel: 2,
          mode: BusChannelMode.stereo,
          lowLatency: false,
        ),
      );
      expect(_apply(settings, _hints, device), isNull);
      expect(controllerAudioAlreadyRouted(settings, _hints, device), isTrue);
    });

    test('treats a default-aliased route as already on a default device', () {
      final device = _device('ddj', 'DDJ-400', isDefault: true);
      final settings = defaultAppSettings();
      expect(controllerAudioAlreadyRouted(settings, _hints, device), isTrue);
    });

    test('returns null when the mapping declares no routes', () {
      expect(
        _apply(
          defaultAppSettings(),
          const ControllerAudioHints(outputNameContains: ['DDJ-400']),
          _device('ddj', 'DDJ-400'),
        ),
        isNull,
      );
    });

    test('records the displaced routes and marks both buses overridden', () {
      final before = defaultAppSettings();
      final updated = _apply(before, _hints, _device('ddj', 'DDJ-400'))!;
      final override = updated.controllerAudioOverride;
      expect(override, isNotNull);
      expect(override!.mappingId, _mappingId);
      expect(override.controllerLabel, _controllerLabel);
      expect(override.outputDeviceId, 'ddj');
      expect(override.previousMasterBus, before.masterBus);
      expect(override.previousPreviewBus, before.previewBus);
      expect(override.previousPreviewEnabled, isFalse);
      expect(override.masterOverridden, isTrue);
      expect(override.previewOverridden, isTrue);
      expect(controllerAudioOverridesBus(updated, master: true), isTrue);
      expect(controllerAudioOverridesBus(updated, master: false), isTrue);
    });

    test('restores the displaced routes and clears the override', () {
      final before = defaultAppSettings();
      final applied = _apply(before, _hints, _device('ddj', 'DDJ-400'))!;
      final restored = restoreControllerAudioOutputs(applied, _mappingId);
      expect(restored, isNotNull);
      expect(restored!.routesRestored, isTrue);
      expect(restored.settings.masterBus, before.masterBus);
      expect(restored.settings.previewBus, before.previewBus);
      expect(restored.settings.previewEnabled, isFalse);
      expect(restored.settings.controllerAudioOverride, isNull);
    });

    test('keeps a bus the user changed by hand while overridden', () {
      final applied = _apply(
        defaultAppSettings(),
        _hints,
        _device('ddj', 'DDJ-400'),
      )!;
      final manual = copyAppSettings(
        applied,
        masterBus: const BusRouteSettings(
          deviceId: 'usb',
          leftChannel: 5,
          rightChannel: 6,
          mode: BusChannelMode.stereo,
          lowLatency: false,
        ),
      );
      final restored = restoreControllerAudioOutputs(manual, _mappingId)!;
      expect(restored.routesRestored, isTrue, reason: 'preview still restores');
      expect(restored.settings.masterBus.deviceId, 'usb');
      expect(restored.settings.previewBus.deviceId, 'default');
    });

    test('clears a stale override without a route change', () {
      final applied = _apply(
        defaultAppSettings(),
        _hints,
        _device('ddj', 'DDJ-400'),
      )!;
      final manual = copyAppSettings(
        applied,
        masterBus: const BusRouteSettings(
          deviceId: 'usb',
          leftChannel: 5,
          rightChannel: 6,
          mode: BusChannelMode.stereo,
          lowLatency: false,
        ),
        previewEnabled: false,
      );
      final restored = restoreControllerAudioOutputs(manual, _mappingId)!;
      expect(restored.routesRestored, isFalse);
      expect(restored.settings.controllerAudioOverride, isNull);
    });

    test('ignores an override owned by another mapping', () {
      final applied = _apply(
        defaultAppSettings(),
        _hints,
        _device('ddj', 'DDJ-400'),
      )!;
      expect(restoreControllerAudioOutputs(applied, 'other'), isNull);
    });
  });
}
