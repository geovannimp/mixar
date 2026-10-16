import 'package:gui_flutter/core/utils/library_list_layout.dart';

const kHistoryIndexWidth = 26.0;

/// Width of the deck badge column.
const kHistoryDeckWidth = 28.0;

/// Leading width of both densities: `#`, deck badge, and the gutters around
/// them. Shared by the row layout and its trailing-meta budget.
const double kHistoryLeadingWidth =
    kHistoryIndexWidth + kRowGutter + kHistoryDeckWidth + kRowGutter;
