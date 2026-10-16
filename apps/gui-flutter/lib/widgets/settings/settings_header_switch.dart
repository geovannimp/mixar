import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/mixar_switch.dart';

/// `MixarSwitch` in the fixed sizing used by card headers and settings rows.
///
/// Keeps the `SizedBox(height: 23) + FittedBox` convention in one place so it
/// cannot drift across panels.
class SettingsHeaderSwitch extends StatelessWidget {
  const new({
    required this.value,
    required this.onChanged,
    super.key,
    this.semanticsLabel,
  });

  final bool value;
  final ValueChanged<bool> onChanged;
  final String? semanticsLabel;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      height: 23,
      child: FittedBox(
        child: MixarSwitch(
          value: value,
          onChanged: onChanged,
          semanticsLabel: semanticsLabel,
        ),
      ),
    );
  }
}
