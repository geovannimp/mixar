import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/misc.dart' show Override;
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/src/rust/api/library.dart';

typedef CollectionTracksLoad = Future<List<LibraryTrackSummary>> Function(
  Ref ref,
);

Override overrideCollectionTracks(CollectionTracksLoad load) =>
    collectionTracksProvider.overrideWith(() => SeededCollectionTracks(load));

Override overrideCollectionTracksValue(List<LibraryTrackSummary> tracks) =>
    overrideCollectionTracks((ref) async => tracks);
