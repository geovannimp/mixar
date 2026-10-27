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

void main() {
  final devices = <OutputDevice>[
    OutputDevice(
      id: 'speakers',
      name: 'Speakers',
      isDefault: true,
      maxChannels: 2,
      defaultSampleRates: Uint32List.fromList([44100, 48000, 96000]),
      mixSampleRate: 48000,
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
                devicesLoading: false,
                onChanged: onChanged,
              ),
            ),
          ),
        ),
      ),
    );
  }

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
