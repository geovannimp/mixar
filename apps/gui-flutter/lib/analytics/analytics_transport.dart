import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/src/rust/api/analytics.dart';
import 'package:path_provider/path_provider.dart';

/// Single analytics client for the whole app. Both the launch event and the
/// Settings toggle must talk to the same instance, otherwise turning analytics
/// off cannot clear the queue the launch event uses.
final analyticsTransportProvider = FutureProvider<AnalyticsTransport>((
  ref,
) async {
  final support = await getApplicationSupportDirectory();
  return await AnalyticsTransport.open(appSupportPath: support.path);
});
