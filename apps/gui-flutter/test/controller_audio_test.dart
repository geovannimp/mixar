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

ControllerAudioOverride? _build(
  AppSettings settings,
  ControllerAudioHints hints,
  OutputDevice device,
) => buildControllerAudioOverride(
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

  group('controllerAudioAlwaysSwitch', () {
    test('recognizes a saved controller preference', () {
      final settings = copyAppSettings(
        defaultAppSettings(),
        alwaysSwitchAudioDeviceIds: ['pioneer.ddj-400'],
      );
      expect(controllerAudioAlwaysSwitch(settings, 'pioneer.ddj-400'), isTrue);
      expect(controllerAudioAlwaysSwitch(settings, 'other'), isFalse);
      expect(controllerAudioAlwaysSwitch(settings, ''), isFalse);
    });
  });

  group('effective routes', () {
    test('uses the live override when one owns the bus', () {
      final override = _build(
        defaultAppSettings(),
        _hints,
        _device('ddj', 'DDJ-400'),
      )!;
      final settings = copyAppSettings(
        defaultAppSettings(),
        controllerAudioOverride: override,
      );
      expect(effectiveMasterBus(settings).deviceId, 'ddj');
      expect(effectivePreviewBus(settings).deviceId, 'ddj');
    });

    test('falls back to the saved routes without an override', () {
      final settings = defaultAppSettings();
      expect(effectiveMasterBus(settings), settings.masterBus);
      expect(effectivePreviewBus(settings), settings.previewBus);
    });
  });

  group('editControllerAudioBus', () {
    test('clears only the edited bus override', () {
      final override = _build(
        defaultAppSettings(),
        _hints,
        _device('ddj', 'DDJ-400'),
      )!;
      final settings = copyAppSettings(
        defaultAppSettings(),
        controllerAudioOverride: override,
      );
      final edited = editControllerAudioBus(
        settings,
        const BusRouteSettings(
          deviceId: 'usb',
          leftChannel: 5,
          rightChannel: 6,
          mode: BusChannelMode.stereo,
          lowLatency: false,
        ),
        master: true,
      );
      expect(edited.masterBus.deviceId, 'usb');
      expect(edited.controllerAudioOverride?.masterOverridden, isFalse);
      expect(edited.controllerAudioOverride?.previewOverridden, isTrue);
      expect(controllerAudioOverridesBus(edited, master: true), isFalse);
      expect(controllerAudioOverridesBus(edited, master: false), isTrue);
    });

    test('clears the whole override when both buses are edited', () {
      final override = _build(
        defaultAppSettings(),
        _hints,
        _device('ddj', 'DDJ-400'),
      )!;
      var settings = copyAppSettings(
        defaultAppSettings(),
        controllerAudioOverride: override,
      );
      settings = editControllerAudioBus(
        settings,
        const BusRouteSettings(
          deviceId: 'usb',
          leftChannel: 5,
          rightChannel: 6,
          mode: BusChannelMode.stereo,
          lowLatency: false,
        ),
        master: true,
      );
      settings = editControllerAudioBus(
        settings,
        const BusRouteSettings(
          deviceId: 'usb',
          leftChannel: 3,
          rightChannel: 4,
          mode: BusChannelMode.stereo,
          lowLatency: false,
        ),
        master: false,
      );
      expect(settings.controllerAudioOverride, isNull);
    });
  });

  group('buildControllerAudioOverride', () {
    test('routes master and headphone buses', () {
      final override = _build(
        defaultAppSettings(),
        _hints,
        _device('ddj', 'DDJ-400'),
      );
      expect(override, isNotNull);
      expect(override!.masterOverridden, isTrue);
      expect(override.masterBus.deviceId, 'ddj');
      expect(override.masterBus.leftChannel, 1);
      expect(override.masterBus.rightChannel, 2);
      expect(override.previewOverridden, isTrue);
      expect(override.previewBus.deviceId, 'ddj');
      expect(override.previewBus.leftChannel, 3);
      expect(override.previewBus.rightChannel, 4);
    });

    test('does not override master for a headphone-only mapping', () {
      final override = _build(
        defaultAppSettings(),
        const ControllerAudioHints(
          outputNameContains: ['DDJ-400'],
          headphoneChannels: ControllerChannelPair(left: 3, right: 4),
        ),
        _device('ddj', 'DDJ-400'),
      );
      expect(override, isNotNull);
      expect(override!.masterOverridden, isFalse);
      expect(override.previewOverridden, isTrue);
      expect(override.previewBus.leftChannel, 3);
    });

    test('uses a declared sample rate', () {
      final override = _build(
        defaultAppSettings(),
        const ControllerAudioHints(
          outputNameContains: ['DDJ-400'],
          masterChannels: ControllerChannelPair(left: 1, right: 2),
          sampleRate: 48000,
        ),
        _device('ddj', 'DDJ-400'),
      );
      expect(override?.masterBus.sampleRate, 48000);
      expect(override?.previewOverridden, isFalse);
    });

    test('offers again when only the saved route matches', () {
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
      // An older build may have persisted the controller into the saved route;
      // a live override must still be created and offered.
      expect(_build(settings, _hints, device), isNotNull);
      expect(
        controllerAudioAlreadyRouted(settings, device, _mappingId),
        isFalse,
      );
    });

    test('sees an existing runtime override as already routed', () {
      final device = _device('ddj', 'DDJ-400');
      final active = _build(defaultAppSettings(), _hints, device)!;
      final settings = copyAppSettings(
        defaultAppSettings(),
        controllerAudioOverride: active,
      );
      expect(_build(settings, _hints, device), isNull);
      expect(
        controllerAudioAlreadyRouted(settings, device, _mappingId),
        isTrue,
      );
    });

    test('offers again when a saved default alias happens to match', () {
      final device = _device('ddj', 'DDJ-400', isDefault: true);
      final settings = defaultAppSettings();
      expect(
        controllerAudioAlreadyRouted(settings, device, _mappingId),
        isFalse,
      );
    });

    test('returns null when the mapping declares no routes', () {
      expect(
        _build(
          defaultAppSettings(),
          const ControllerAudioHints(outputNameContains: ['DDJ-400']),
          _device('ddj', 'DDJ-400'),
        ),
        isNull,
      );
    });

    test('marks both buses for the Settings badges', () {
      final override = _build(
        defaultAppSettings(),
        _hints,
        _device('ddj', 'DDJ-400'),
      )!;
      final settings = copyAppSettings(
        defaultAppSettings(),
        controllerAudioOverride: override,
      );
      expect(controllerAudioOverridesBus(settings, master: true), isTrue);
      expect(controllerAudioOverridesBus(settings, master: false), isTrue);
    });
  });
}
