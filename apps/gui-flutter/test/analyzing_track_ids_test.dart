import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/library/providers.dart';

void main() {
  test('analyzing tracks keep loaders when a second analyze starts', () {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    final notifier = container.read(analyzingTrackIdsProvider.notifier);

    notifier.add('track-a');
    expect(container.read(analyzingTrackIdsProvider), {'track-a'});

    notifier.add('track-b');
    expect(container.read(analyzingTrackIdsProvider), {'track-a', 'track-b'});

    notifier.clearIf('track-a');
    expect(container.read(analyzingTrackIdsProvider), {'track-b'});
  });
}
