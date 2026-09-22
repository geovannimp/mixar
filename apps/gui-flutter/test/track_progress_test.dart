import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/library/providers.dart';

void main() {
  test('TrackProgressInfo labels phases and percent', () {
    expect(const TrackProgressInfo(phase: 'analyze').label, 'Analyzing');
    expect(
      const TrackProgressInfo(phase: 'stems_separate', fraction: 0.42).label,
      'Separating stems 42%',
    );
  });

  test('stem generating set tracks in-flight jobs only', () {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    final notifier = container.read(stemGeneratingTrackIdsProvider.notifier);
    notifier.setGenerating('a', true);
    expect(container.read(stemGeneratingTrackIdsProvider), {'a'});
    notifier.setGenerating('a', false);
    expect(container.read(stemGeneratingTrackIdsProvider), isEmpty);
  });
}
