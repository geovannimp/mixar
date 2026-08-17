import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/shell/desktop.dart';
import 'package:gui_flutter/src/rust/api/controller.dart';
import 'package:path/path.dart' as p;
import 'package:path_provider/path_provider.dart';

/// Starts once on desktop after engine+library buses exist.
/// MIDI is optional: start failure is reported and the rest of the shell keeps running.
final controllerTransportProvider = FutureProvider<ControllerTransport?>((
  ref,
) async {
  if (!isDesktopWindow) {
    return null;
  }
  final library = await ref.watch(libraryTransportProvider.future);
  final engine = await ref.watch(engineTransportProvider.future);
  if (engine == null) {
    return null;
  }
  final support = await getApplicationSupportDirectory();
  final mappingsDir = p.join(support.path, 'mappings');
  try {
    final controller = await ControllerTransport.start(
      engineBuses: await engine.buses(),
      libraryBuses: await library.buses(),
      mappingsDir: mappingsDir,
    );
    ref.keepAlive();
    ref.onDispose(controller.dispose);
    return controller;
  } catch (e, st) {
    FlutterError.reportError(FlutterErrorDetails(exception: e, stack: st));
    return null;
  }
});

final controllerMappingsProvider = FutureProvider<List<ControllerMappingInfo>>((
  ref,
) async {
  final transport = await ref.watch(controllerTransportProvider.future);
  if (transport == null) {
    return const [];
  }
  return transport.listMappings();
});

final controllerDevicesProvider = FutureProvider<List<ControllerDeviceInfo>>((
  ref,
) async {
  final transport = await ref.watch(controllerTransportProvider.future);
  if (transport == null) {
    return const [];
  }
  return transport.listDevices();
});
