import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/settings/settings_providers.dart';
import 'package:gui_flutter/shell/desktop.dart';
import 'package:gui_flutter/src/rust/api/controller.dart';
import 'package:path/path.dart' as p;
import 'package:path_provider/path_provider.dart';

/// Starts once on desktop after engine+library buses exist.
/// MIDI is optional: start failure is reported and the rest of the shell keeps running.
/// Rebuilt when app settings (trusted controllers) change via invalidate.
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
  // Read once per (re)start — invalidate this provider when trusts change.
  final settings = await ref.read(appSettingsProvider.future);
  final support = await getApplicationSupportDirectory();
  final mappingsDir = p.join(support.path, 'mappings');
  try {
    final controller = await ControllerTransport.start(
      engineBuses: await engine.buses(),
      libraryBuses: await library.buses(),
      mappingsDir: mappingsDir,
      trustedDeviceIds: List<String>.from(settings.trustedControllerDeviceIds),
    );
    ref.keepAlive();
    ref.onDispose(controller.dispose);
    return controller;
  } catch (e, st) {
    FlutterError.reportError(FlutterErrorDetails(exception: e, stack: st));
    return null;
  }
});

/// Catalog of mapping files. Listed once when the controller host starts.
final controllerMappingsProvider = FutureProvider<List<ControllerMappingInfo>>((
  ref,
) async {
  ref.keepAlive();
  final transport = await ref.watch(controllerTransportProvider.future);
  if (transport == null) {
    return const [];
  }
  final rows = await transport.listMappings();
  ref
      .read(attachedMappingIdsProvider.notifier)
      .sync(rows.where((row) => row.attached).map((row) => row.id));
  return rows;
});

/// Live MIDI ports. Reloaded from attach/detach bus events, not by re-listing mappings.
final controllerDevicesProvider = FutureProvider<List<ControllerDeviceInfo>>((
  ref,
) async {
  final transport = await ref.watch(controllerTransportProvider.future);
  if (transport == null) {
    return const [];
  }
  return transport.listDevices();
});

class AttachedMappingIds extends Notifier<Set<String>> {
  @override
  Set<String> build() => {};

  void sync(Iterable<String> ids) => state = ids.toSet();
}

final attachedMappingIdsProvider =
    NotifierProvider<AttachedMappingIds, Set<String>>(AttachedMappingIds.new);

Future<void> syncAttachedMappingIds(WidgetRef ref) async {
  final transport = ref.read(controllerTransportProvider).value;
  if (transport == null) {
    return;
  }
  final rows = await transport.listMappings();
  ref
      .read(attachedMappingIdsProvider.notifier)
      .sync(rows.where((row) => row.attached).map((row) => row.id));
}
