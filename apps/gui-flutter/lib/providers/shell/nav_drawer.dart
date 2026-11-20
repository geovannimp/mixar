import 'package:flutter_riverpod/flutter_riverpod.dart';

/// Whether the pages menu route is open.
///
/// The titlebar stays clickable while it is open, so the hamburger has to be a
/// real toggle: a second press closes the menu instead of pushing it again.
class NavDrawerOpen extends Notifier<bool> {
  @override
  bool build() => false;

  void set({required bool value}) {
    if (state != value) {
      state = value;
    }
  }
}

final navDrawerOpenProvider = NotifierProvider<NavDrawerOpen, bool>(
  NavDrawerOpen.new,
);
