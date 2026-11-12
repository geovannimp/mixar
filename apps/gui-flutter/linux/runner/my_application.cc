#include "my_application.h"

#include <flutter_linux/flutter_linux.h>
#ifdef GDK_WINDOWING_X11
#include <gdk/gdkx.h>
#endif

#include "flutter/generated_plugin_registrant.h"

struct _MyApplication {
  GtkApplication parent_instance;
  char** dart_entrypoint_arguments;
};

G_DEFINE_TYPE(MyApplication, my_application, GTK_TYPE_APPLICATION)

// Called when first Flutter frame received.
static void first_frame_cb(MyApplication* self, FlView* view) {
  gtk_widget_show(gtk_widget_get_toplevel(GTK_WIDGET(view)));
}

// Implements GApplication::activate.
static void my_application_activate(GApplication* application) {
  MyApplication* self = MY_APPLICATION(application);
  GtkWindow* window =
      GTK_WINDOW(gtk_application_window_new(GTK_APPLICATION(application)));

  // RGBA visual so transparent Flutter corners composite with the DE shadow
  // (needed with TitleBarStyle.hidden + ClipRRect rounded chrome).
  // window_manager setBackgroundColor alone is not enough on Linux:
  // https://github.com/leanflutter/window_manager/issues/179
  gtk_widget_set_app_paintable(GTK_WIDGET(window), TRUE);
  {
    GdkScreen* screen = gtk_window_get_screen(window);
    GdkVisual* visual = gdk_screen_get_rgba_visual(screen);
    if (visual != nullptr && gdk_screen_is_composited(screen)) {
      gtk_widget_set_visual(GTK_WIDGET(window), visual);
    }
  }

  // Use a header bar when running in GNOME as this is the common style used
  // by applications and is the setup most users will be using (e.g. Ubuntu
  // desktop).
  // If running on X and not using GNOME then just use a traditional title bar
  // in case the window manager does more exotic layout, e.g. tiling.
  // If running on Wayland assume the header bar will work (may need changing
  // if future cases occur).
  // window_manager TitleBarStyle.hidden removes this at runtime for our CSD.
  gboolean use_header_bar = TRUE;
#ifdef GDK_WINDOWING_X11
  GdkScreen* screen = gtk_window_get_screen(window);
  if (GDK_IS_X11_SCREEN(screen)) {
    const gchar* wm_name = gdk_x11_screen_get_window_manager_name(screen);
    if (g_strcmp0(wm_name, "GNOME Shell") != 0) {
      use_header_bar = FALSE;
    }
  }
#endif
  if (use_header_bar) {
    GtkHeaderBar* header_bar = GTK_HEADER_BAR(gtk_header_bar_new());
    gtk_widget_show(GTK_WIDGET(header_bar));
    gtk_header_bar_set_title(header_bar, "Mixar");
    gtk_header_bar_set_show_close_button(header_bar, TRUE);
    gtk_window_set_titlebar(window, GTK_WIDGET(header_bar));
  } else {
    gtk_window_set_title(window, "Mixar");
  }

  // The runner owns the window geometry. It used to be applied from Dart
  // through window_manager's WindowOptions, but on Flutter 3.50 that adds the
  // window frame margin again on every resize: 1280x800 grew to 1544x1064 and
  // then 1808x1328 while the FlView stayed 1280x800, inset 132px per side, so
  // the UI ended up in the bottom-left of a mostly transparent window. Sizing
  // here keeps window, surface and view in agreement (verified 1280x800 @0,0).
  gtk_window_set_default_size(window, 1280, 800);
  gtk_window_set_position(window, GTK_WIN_POS_CENTER);
  // window_manager WindowOptions.minimumSize uses gdk_window_set_geometry_hints,
  // which Wayland compositors ignore. Set it on the GtkWindow instead:
  // https://github.com/leanflutter/window_manager/issues/538
  GdkGeometry hints = {};
  hints.min_width = 1024;
  hints.min_height = 768;
  gtk_window_set_geometry_hints(window, nullptr, &hints, GDK_HINT_MIN_SIZE);

  g_autoptr(FlDartProject) project = fl_dart_project_new();
  // Flutter 3.50's engine exports libunwind's `_Unwind_*` / `unw_*` symbols
  // from libflutter_linux_gtk.so (3.47.7 exported none of them). Rust's
  // `std::backtrace` then resolves `_Unwind_GetIP` into the engine, which
  // unwraps a libgcc_s frame context with libunwind and segfaults. anyhow
  // captures a backtrace for every error, so an ordinary error path — e.g. a
  // saved audio device that no longer resolves, which is meant to fall back to
  // the system default — killed the app at startup. Keep std from capturing
  // backtraces here; an explicit RUST_LIB_BACKTRACE=1 still wins (and will
  // crash) if someone needs to debug one.
  g_setenv("RUST_LIB_BACKTRACE", "0", /*overwrite=*/FALSE);
  // Impeller is the default backend (Skia here was only ever a workaround for
  // the Linux compositor bug flutter/flutter#195885, fixed upstream).
  fl_dart_project_set_dart_entrypoint_arguments(
      project, self->dart_entrypoint_arguments);

  FlView* view = fl_view_new(project);
  GdkRGBA background_color;
  // Transparent so rounded ClipRRect corners aren't filled with opaque black.
  gdk_rgba_parse(&background_color, "#00000000");
  fl_view_set_background_color(view, &background_color);
  gtk_widget_show(GTK_WIDGET(view));
  gtk_container_add(GTK_CONTAINER(window), GTK_WIDGET(view));

  // No GTK decorations: the app draws its own title bar and rounded chrome
  // (window_manager TitleBarStyle.hidden). This has to happen after GTK's own
  // header bar is configured and before the window is realized, otherwise GTK
  // keeps its CSD shadow margins.
  //
  // Those margins are the reason the UI was landing in the bottom left. With
  // CSD the toplevel GdkWindow -- the surface Flutter renders into -- is larger
  // than the view: measured 1332x852 holding a 1280x800 FlView at (26,23).
  // Flutter draws the 1280x800 scene into that surface without accounting for
  // the margin, so the scene sits offset toward the bottom left (GL origin) and
  // the margin stays transparent -- the desktop shows through, because the
  // window is RGBA. Decorated=false makes window, surface and view agree:
  // 1280x800 @ 0,0.
  gtk_window_set_decorated(window, FALSE);

  // Show the window when Flutter renders.
  // Requires the view to be realized so we can start rendering.
  g_signal_connect_swapped(view, "first-frame", G_CALLBACK(first_frame_cb),
                           self);
  gtk_widget_realize(GTK_WIDGET(view));

  fl_register_plugins(FL_PLUGIN_REGISTRY(view));

  gtk_widget_grab_focus(GTK_WIDGET(view));
}

// Implements GApplication::local_command_line.
static gboolean my_application_local_command_line(GApplication* application,
                                                  gchar*** arguments,
                                                  int* exit_status) {
  MyApplication* self = MY_APPLICATION(application);
  // Strip out the first argument as it is the binary name.
  self->dart_entrypoint_arguments = g_strdupv(*arguments + 1);

  g_autoptr(GError) error = nullptr;
  if (!g_application_register(application, nullptr, &error)) {
    g_warning("Failed to register: %s", error->message);
    *exit_status = 1;
    return TRUE;
  }

  g_application_activate(application);
  *exit_status = 0;

  return TRUE;
}

// Implements GApplication::startup.
static void my_application_startup(GApplication* application) {
  // MyApplication* self = MY_APPLICATION(object);

  // Perform any actions required at application startup.

  G_APPLICATION_CLASS(my_application_parent_class)->startup(application);
}

// Implements GApplication::shutdown.
static void my_application_shutdown(GApplication* application) {
  // MyApplication* self = MY_APPLICATION(object);

  // Perform any actions required at application shutdown.

  G_APPLICATION_CLASS(my_application_parent_class)->shutdown(application);
}

// Implements GObject::dispose.
static void my_application_dispose(GObject* object) {
  MyApplication* self = MY_APPLICATION(object);
  g_clear_pointer(&self->dart_entrypoint_arguments, g_strfreev);
  G_OBJECT_CLASS(my_application_parent_class)->dispose(object);
}

static void my_application_class_init(MyApplicationClass* klass) {
  G_APPLICATION_CLASS(klass)->activate = my_application_activate;
  G_APPLICATION_CLASS(klass)->local_command_line =
      my_application_local_command_line;
  G_APPLICATION_CLASS(klass)->startup = my_application_startup;
  G_APPLICATION_CLASS(klass)->shutdown = my_application_shutdown;
  G_OBJECT_CLASS(klass)->dispose = my_application_dispose;
}

static void my_application_init(MyApplication* self) {}

MyApplication* my_application_new() {
  // Set the program name to the application ID, which helps various systems
  // like GTK and desktop environments map this running application to its
  // corresponding .desktop file. This ensures better integration by allowing
  // the application to be recognized beyond its binary name.
  g_set_prgname(APPLICATION_ID);

  return MY_APPLICATION(g_object_new(my_application_get_type(),
                                     "application-id", APPLICATION_ID, "flags",
                                     G_APPLICATION_NON_UNIQUE, nullptr));
}
