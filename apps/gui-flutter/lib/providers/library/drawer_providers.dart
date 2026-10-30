import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/src/rust/api/settings.dart' show AppSettings;

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
/// the header toggle is the way back. A deck load closes it too, whichever
/// path asked for the load (`loadPayloadOntoDeck`).
class LibraryDrawerOpen extends Notifier<bool> {
  @override
  bool build() => false;

  void set(bool open) => state = open;

  void toggle() => state = !state;
}

final libraryDrawerOpenProvider = NotifierProvider<LibraryDrawerOpen, bool>(
  LibraryDrawerOpen.new,
);

/// Asks the docked mixer layout to bring the library pane into view.
///
/// Docked mode always renders the library, but a short window leaves it below
/// the fold; a deck action that wants the library bumps this and the mixer page
/// scrolls to it. Drawer mode opens the drawer route instead.
class LibraryReveal extends Notifier<int> {
  @override
  int build() => 0;

  void reveal() => state = state + 1;
}

final libraryRevealProvider = NotifierProvider<LibraryReveal, int>(
  LibraryReveal.new,
);

/// Brings the library into view for a deck action such as a cover click.
///
/// Whichever layout is on: the drawer opens, or the docked pane scrolls to the
/// library. Loading itself still happens in the library (drag or double-click
/// onto the deck).
void openLibrary(WidgetRef ref) {
  if (_drawerMode(ref.read(appSettingsProvider).asData?.value)) {
    ref.read(libraryDrawerOpenProvider.notifier).set(true);
    return;
  }
  ref.read(libraryRevealProvider.notifier).reveal();
}

/// Whether the library lives in a drawer (Settings → UI). Docked mode has
/// nothing to open or dismiss.
bool _drawerMode(AppSettings? settings) =>
    settings?.libraryDrawerEnabled ?? false;

bool _libraryLivesInDrawer(Ref ref) =>
    _drawerMode(ref.read(appSettingsProvider).asData?.value);

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
