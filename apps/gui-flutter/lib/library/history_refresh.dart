import 'package:flutter_riverpod/flutter_riverpod.dart';

class HistoryRefreshTick extends Notifier<int> {
  @override
  int build() => 0;

  void bump() => state++;
}

final historyRefreshTickProvider =
    NotifierProvider<HistoryRefreshTick, int>(HistoryRefreshTick.new);
