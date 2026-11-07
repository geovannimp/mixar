import 'dart:typed_data';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/src/rust/api/engine.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_bus_route_fields.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

OutputDevice _device({
  required String id,
  required String name,
  bool isDefault = false,
  int maxChannels = 2,
  required List<int> rates,
  required List<SampleRateRange> ranges,
  int? mixRate,
}) => OutputDevice(
  id: id,
  name: name,
  isDefault: isDefault,
  maxChannels: maxChannels,
  defaultSampleRates: Uint32List.fromList(rates),
  sampleRateRanges: ranges,
  mixSampleRate: mixRate,
);

void main() {
  final devices = <OutputDevice>[
    _device(
      id: 'speakers',
      name: 'Speakers',
      isDefault: true,
      rates: [44100, 48000, 96000],
      ranges: const [
        SampleRateRange(min: 44100, max: 48000),
        SampleRateRange(min: 96000, max: 96000),
      ],
      mixRate: 48000,
    ),
    _device(
      id: 'headphones',
      name: 'Headphones',
      rates: [44100],
      ranges: const [SampleRateRange(min: 44100, max: 44100)],
      mixRate: 44100,
    ),
    // Advertises only its span endpoints; 96 kHz is inside the span.
    _device(
      id: 'interface',
      name: 'Interface',
      rates: [44100, 192000],
      ranges: const [SampleRateRange(min: 44100, max: 192000)],
      mixRate: 48000,
    ),
    // Mixes at a rate it does not list as a supported endpoint.
    _device(
      id: 'dac',
      name: 'DAC',
      rates: [44100, 48000],
      ranges: const [SampleRateRange(min: 44100, max: 48000)],
      mixRate: 96000,
    ),
  ];
  const baseRoute = BusRouteSettings(
    deviceId: 'speakers',
    leftChannel: 1,
    rightChannel: 2,
    mode: BusChannelMode.stereo,
    lowLatency: false,
  );

  Future<void> pumpFields(
    WidgetTester tester, {
    required ValueChanged<BusRouteSettings> onChanged,
    BusRouteSettings route = baseRoute,
    bool devicesLoading = false,
    BusRouteSettings? siblingBus,
  }) {
    return tester.pumpWidget(
      ProviderScope(
        overrides: [
          appSettingsProvider.overrideWith((ref) async => defaultAppSettings()),
        ],
        child: mixarTestMaterialApp(
          theme: MixarThemeData.dark(),
          home: Scaffold(
            body: SizedBox(
              width: 520,
              child: SettingsBusRouteFields(
                route: route,
                devices: devices,
                devicesLoading: devicesLoading,
                siblingBus: siblingBus,
                onChanged: onChanged,
              ),
            ),
          ),
        ),
      ),
    );
  }

  test('hides the backend default alias from the device list', () {
    final alias = _device(
      id: 'cpal:pipewire:output_default',
      name: 'System default',
      isDefault: true,
      rates: [48000],
      ranges: const [SampleRateRange(min: 48000, max: 48000)],
      mixRate: 48000,
    );
    final visible = selectableAudioDevices([
      ...devices,
      alias,
    ], 'System default');
    expect(visible.where((device) => device.name == 'System default'), isEmpty);
    expect(visible, hasLength(devices.length));
  });

  testWidgets('the default rate option names the device mix rate', (
    tester,
  ) async {
    await pumpFields(tester, onChanged: (_) {});
    expect(find.text('Device default (48000 Hz)'), findsOneWidget);
  }, semanticsEnabled: false);

  testWidgets('choosing a rate reports it with the rest of the route', (
    tester,
  ) async {
    BusRouteSettings? next;
    await pumpFields(tester, onChanged: (route) => next = route);

    await tester.tap(find.text('Device default (48000 Hz)'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('44100 Hz').last);
    await tester.pumpAndSettle();

    expect(next, isNotNull);
    expect(next!.sampleRate, 44100);
    expect(next!.deviceId, 'speakers');
    expect(next!.leftChannel, 1);
    expect(next!.rightChannel, 2);
  }, semanticsEnabled: false);

  testWidgets('an explicit rate survives changing another field', (
    tester,
  ) async {
    BusRouteSettings? next;
    await pumpFields(
      tester,
      route: const BusRouteSettings(
        deviceId: 'speakers',
        leftChannel: 1,
        rightChannel: 2,
        mode: BusChannelMode.stereo,
        sampleRate: 96000,
        lowLatency: false,
      ),
      onChanged: (route) => next = route,
    );
    expect(find.text('96000 Hz'), findsOneWidget);

    // Switching channel mode keeps the chosen rate on the bus.
    await tester.tap(find.text('Stereo pair'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Mono (fold L+R)').last);
    await tester.pumpAndSettle();

    expect(next?.sampleRate, 96000);
    expect(next?.mode, BusChannelMode.mono);
  }, semanticsEnabled: false);

  testWidgets('low latency is a bus flag and leaves the rate alone', (
    tester,
  ) async {
    BusRouteSettings? next;
    await pumpFields(
      tester,
      route: const BusRouteSettings(
        deviceId: 'speakers',
        leftChannel: 1,
        rightChannel: 2,
        mode: BusChannelMode.stereo,
        sampleRate: 96000,
        lowLatency: false,
      ),
      onChanged: (route) => next = route,
    );

    await tester.tap(find.text('Low latency'));
    await tester.pumpAndSettle();

    expect(next?.lowLatency, isTrue);
    expect(next?.sampleRate, 96000);
  }, semanticsEnabled: false);

  testWidgets('switching device drops a rate the new device lacks', (
    tester,
  ) async {
    BusRouteSettings? next;
    await pumpFields(
      tester,
      route: const BusRouteSettings(
        deviceId: 'speakers',
        leftChannel: 1,
        rightChannel: 2,
        mode: BusChannelMode.stereo,
        sampleRate: 96000,
        lowLatency: false,
      ),
      onChanged: (route) => next = route,
    );

    await tester.tap(find.text('System default'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Headphones').last);
    await tester.pumpAndSettle();

    expect(next?.deviceId, 'headphones');
    // Headphones only advertise 44.1 kHz; keeping 96 kHz would fail the start.
    expect(next?.sampleRate, isNull);
  }, semanticsEnabled: false);

  testWidgets('the rate select hints while devices are loading', (
    tester,
  ) async {
    await pumpFields(tester, onChanged: (_) {}, devicesLoading: true);
    expect(find.text('Loading rates for this device…'), findsOneWidget);
  }, semanticsEnabled: false);

  testWidgets('an explicit rate inside the new device span survives', (
    tester,
  ) async {
    BusRouteSettings? next;
    await pumpFields(
      tester,
      route: const BusRouteSettings(
        deviceId: 'speakers',
        leftChannel: 1,
        rightChannel: 2,
        mode: BusChannelMode.stereo,
        sampleRate: 96000,
        lowLatency: false,
      ),
      onChanged: (route) => next = route,
    );

    await tester.tap(find.text('System default'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Interface').last);
    await tester.pumpAndSettle();

    // 96 kHz is inside the interface's 44.1–192 kHz span even though it is not
    // one of the advertised endpoints.
    expect(next?.deviceId, 'interface');
    expect(next?.sampleRate, 96000);
  }, semanticsEnabled: false);

  testWidgets(
    'the default label follows the sibling bus that pins the stream',
    (tester) async {
      await pumpFields(
        tester,
        route: const BusRouteSettings(
          deviceId: 'speakers',
          leftChannel: 3,
          rightChannel: 4,
          mode: BusChannelMode.stereo,
          lowLatency: false,
        ),
        // Master on the same device pins 44.1 kHz, so the shared stream runs there
        // even though this bus follows the device default.
        siblingBus: const BusRouteSettings(
          deviceId: 'speakers',
          leftChannel: 1,
          rightChannel: 2,
          mode: BusChannelMode.stereo,
          sampleRate: 44100,
          lowLatency: false,
        ),
        onChanged: (_) {},
      );
      expect(find.text('Device default (44100 Hz)'), findsOneWidget);
    },
    semanticsEnabled: false,
  );

  testWidgets('the default label shows the device mix rate', (tester) async {
    await pumpFields(
      tester,
      route: const BusRouteSettings(
        deviceId: 'dac',
        leftChannel: 1,
        rightChannel: 2,
        mode: BusChannelMode.stereo,
        lowLatency: false,
      ),
      onChanged: (_) {},
    );
    expect(find.text('Device default (96000 Hz)'), findsOneWidget);
  }, semanticsEnabled: false);

  testWidgets('the bus can go back to the device default rate', (tester) async {
    BusRouteSettings? next;
    await pumpFields(
      tester,
      route: const BusRouteSettings(
        deviceId: 'speakers',
        leftChannel: 1,
        rightChannel: 2,
        mode: BusChannelMode.stereo,
        sampleRate: 96000,
        lowLatency: false,
      ),
      onChanged: (route) => next = route,
    );

    await tester.tap(find.text('96000 Hz'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Device default (48000 Hz)').last);
    await tester.pumpAndSettle();

    expect(next?.sampleRate, isNull);
    expect(next?.deviceId, 'speakers');
  }, semanticsEnabled: false);
}
