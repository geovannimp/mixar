import 'package:flutter_riverpod/flutter_riverpod.dart';

/// Smallest drawer edge in px; the rendered size also clamps to the mixer
/// extent, so a window narrower than this still shows a usable drawer.
const double kLibraryDrawerMinExtent = 240;

/// Session-local drawer extent per axis (px), applied along the docked edge:
/// `width` for the left/right edges, `height` for the top/bottom ones.
class LibraryDrawerSize {
  const new({this.width = 480, this.height = 480});

  final double width;
  final double height;

  LibraryDrawerSize copyWith({double? width, double? height}) =>
      LibraryDrawerSize(
        width: width ?? this.width,
        height: height ?? this.height,
      );
}

class LibraryDrawerSizeNotifier extends Notifier<LibraryDrawerSize> {
  @override
  LibraryDrawerSize build() => const LibraryDrawerSize();

  void setWidth(double width) => state = state.copyWith(width: width);

  void setHeight(double height) => state = state.copyWith(height: height);
}

final libraryDrawerSizeProvider =
    NotifierProvider<LibraryDrawerSizeNotifier, LibraryDrawerSize>(
      LibraryDrawerSizeNotifier.new,
    );

/// Whether the library drawer is open. Session-local and starts closed on each
/// launch, like the `panes` split sizes.
class LibraryDrawerOpen extends Notifier<bool> {
  @override
  bool build() => false;

  void set(bool open) => state = open;

  void toggle() => state = !state;
}

final libraryDrawerOpenProvider = NotifierProvider<LibraryDrawerOpen, bool>(
  LibraryDrawerOpen.new,
);

/// A library track drag is in flight. The drawer slides off its edge while this
/// is set so a deck drop zone is never covered at drop time; the drag source
/// stays mounted (only translated), so the native drag is not cancelled.
class LibraryTrackDragActive extends Notifier<bool> {
  @override
  bool build() => false;

  void begin() => state = true;

  void end() => state = false;
}

final libraryTrackDragActiveProvider =
    NotifierProvider<LibraryTrackDragActive, bool>(LibraryTrackDragActive.new);

/// What the drawer widget actually renders: open, unless a track drag hides it.
final libraryDrawerVisibleProvider = Provider<bool>(
  (ref) =>
      ref.watch(libraryDrawerOpenProvider) &&
      !ref.watch(libraryTrackDragActiveProvider),
);
