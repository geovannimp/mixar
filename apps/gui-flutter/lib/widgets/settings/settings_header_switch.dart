import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/mixar_switch.dart';

/// `MixarSwitch` in the fixed sizing used by card/section headers.
///
/// Keeps the `SizedBox(height: 23) + FittedBox` convention in one place so it
/// cannot drift across panels. Non-header settings rows use
/// `SettingsCheckbox` instead.
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
