import 'package:analysis_server_plugin/plugin.dart';
import 'package:analysis_server_plugin/registry.dart';
import 'package:gui_flutter_lints/src/prefer_single_widget_per_file.dart';

final plugin = GuiFlutterLintsPlugin();

/// Local analyzer plugin for `apps/gui-flutter`.
///
/// Mirrors the `analysis_server_plugin` shape used by `riverpod_lint` 3.x.
class GuiFlutterLintsPlugin extends Plugin {
  @override
  String get name => 'gui_flutter_lints';

  @override
  void register(PluginRegistry registry) {
    registry.registerWarningRule(PreferSingleWidgetPerFile());
  }
}
