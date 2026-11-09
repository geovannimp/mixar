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

/// Whether the mixer has forced the library into the drawer because the body
/// left too little room to dock it.
///
/// The page sets this from the solved split; the header uses it to offer the
/// drawer toggle, and [openLibrary] uses it to decide that a deck action has a
/// drawer to open.
class MixerLibraryForcedDrawer extends Notifier<bool> {
  @override
  bool build() => false;

  void set(bool value) {
    if (state != value) {
      state = value;
    }
  }
}

final mixerLibraryForcedDrawerProvider =
    NotifierProvider<MixerLibraryForcedDrawer, bool>(
      MixerLibraryForcedDrawer.new,
    );

/// Brings the library into view for a deck action such as a cover click.
///
/// Whichever layout is on: the drawer opens, or the docked library is already
/// on screen and there is nothing to reveal. Loading itself still happens in
/// the library (a row is dragged, or its Load to deck menu used, onto the
/// deck).
void openLibrary(WidgetRef ref) {
  final settings = ref.read(appSettingsProvider).asData?.value;
  if (!_drawerAvailable(settings, ref.read(mixerLibraryForcedDrawerProvider))) {
    return;
  }
  ref.read(libraryDrawerOpenProvider.notifier).set(true);
}

/// The one place the drawer rule lives: the setting, or the mixer forcing it
/// when the body left too little room to dock the library.
bool _drawerAvailable(AppSettings? settings, bool forced) =>
    (settings?.libraryDrawerEnabled ?? false) || forced;

/// Whether the library lives in a drawer. Docked mode has nothing to open or
/// dismiss.
bool _libraryLivesInDrawer(Ref ref) => _drawerAvailable(
  ref.read(appSettingsProvider).asData?.value,
  ref.read(mixerLibraryForcedDrawerProvider),
);

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
