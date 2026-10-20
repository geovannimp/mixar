import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';

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
///
/// A track drag closes it (see `showLibraryDrawer`) and nothing reopens it —
/// the header toggle is the way back.
class LibraryDrawerOpen extends Notifier<bool> {
  @override
  bool build() => false;

  void set(bool open) => state = open;

  void toggle() => state = !state;
}

final libraryDrawerOpenProvider = NotifierProvider<LibraryDrawerOpen, bool>(
  LibraryDrawerOpen.new,
);

/// Whether the library lives in a drawer (Settings → UI). Docked mode has
/// nothing to open or dismiss.
bool _libraryLivesInDrawer(Ref ref) =>
    ref.read(appSettingsProvider).asData?.value.libraryDrawerEnabled ?? false;

/// Brings the drawer up for a forward controller focus step — the DDJ-400's
/// library knob press — so its rotary then moves a cursor the user can see.
void revealLibraryDrawer(Ref ref) {
  if (!_libraryLivesInDrawer(ref)) {
    return;
  }
  ref.read(libraryDrawerOpenProvider.notifier).set(true);
}

/// Dismisses the drawer for a backward focus step — SHIFT + the library knob
/// press. Returns whether the drawer took the step, so the caller does not also
/// move a focus cursor in a surface that is closing.
bool dismissLibraryDrawer(Ref ref) {
  if (!_libraryLivesInDrawer(ref)) {
    return false;
  }
  ref.read(libraryDrawerOpenProvider.notifier).set(false);
  return true;
}

/// A library track drag is in flight.
///
/// The drawer tucks itself off its edge while this is set — without unmounting,
/// because the row being dragged lives inside it and the native drag dies with
/// its source — so the deck being dragged to is never covered at drop time. The
/// drag's end closes the drawer.
class LibraryTrackDragActive extends Notifier<bool> {
  @override
  bool build() => false;

  void begin() => state = true;

  void end() => state = false;
}

final libraryTrackDragActiveProvider =
    NotifierProvider<LibraryTrackDragActive, bool>(LibraryTrackDragActive.new);
