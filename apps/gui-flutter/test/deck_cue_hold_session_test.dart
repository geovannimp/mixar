import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/mixer/deck_cue_hold_session.dart';

void main() {
  test('end waits for begin and is ignored without successful begin', () async {
    final session = DeckCueHoldSession();
    final log = <String>[];
    final beginStarted = Completer<void>();
    final beginRelease = Completer<void>();

    session
      ..begin(() async {
        log.add('begin-start');
        beginStarted.complete();
        await beginRelease.future;
        log.add('begin-done');
      })
      ..end(() async {
        log.add('end');
      });

    await beginStarted.future;
    expect(log, ['begin-start']);

    beginRelease.complete();
    await session.drain();
    expect(log, ['begin-start', 'begin-done', 'end']);
  });

  test('failed begin drops a following end', () async {
    final session = DeckCueHoldSession();
    final log = <String>[];

    session
      ..begin(() async {
        log.add('begin');
        throw StateError('begin failed');
      })
      ..end(() async {
        log.add('end');
      });

    await session.drain();
    expect(log, ['begin']);
  });

  test('disposeEnd awaits chained end', () async {
    final session = DeckCueHoldSession();
    final log = <String>[];
    final beginRelease = Completer<void>();

    session.begin(() async {
      log.add('begin');
      await beginRelease.future;
    });
    beginRelease.complete();
    await session.disposeEnd(() async {
      log.add('end');
    });
    expect(log, ['begin', 'end']);
  });

  test('disposeEnd propagates end failures', () async {
    final session = DeckCueHoldSession()..begin(() async {});
    await session.drain();

    await expectLater(
      session.disposeEnd(() async {
        throw StateError('end failed');
      }),
      throwsA(isA<StateError>()),
    );

    // `_began` stays true on failure so a later end can still release; after
    // that retry the session accepts a fresh begin/end cycle.
    final log = <String>[];
    session.end(() async {
      log.add('end-retry');
    });
    await session.drain();
    session
      ..begin(() async {
        log.add('begin');
      })
      ..end(() async {
        log.add('end');
      });
    await session.drain();
    expect(log, ['end-retry', 'begin', 'end']);
  });

  test('disposeEnd is a no-op without prior begin', () async {
    final session = DeckCueHoldSession();
    var ended = false;

    await session.disposeEnd(() async {
      ended = true;
    });
    expect(ended, isFalse);
  });

  test('session supports multiple begin/end cycles', () async {
    final session = DeckCueHoldSession();
    final log = <String>[];

    session
      ..begin(() async {
        log.add('begin-1');
      })
      ..end(() async {
        log.add('end-1');
      });
    await session.drain();

    session
      ..begin(() async {
        log.add('begin-2');
      })
      ..end(() async {
        log.add('end-2');
      });
    await session.drain();

    expect(log, ['begin-1', 'end-1', 'begin-2', 'end-2']);
  });

  test('re-entrant begin is ignored while hold is active', () async {
    final session = DeckCueHoldSession();
    final log = <String>[];
    final beginRelease = Completer<void>();

    session
      ..begin(() async {
        log.add('begin-1');
        await beginRelease.future;
      })
      ..begin(() async {
        log.add('begin-2');
      });
    beginRelease.complete();
    session.end(() async {
      log.add('end');
    });
    await session.drain();

    expect(log, ['begin-1', 'end']);
  });

  test('second end/disposeEnd is ignored while end is queued', () async {
    final session = DeckCueHoldSession()..begin(() async {});
    await session.drain();

    final log = <String>[];
    final endRelease = Completer<void>();
    session
      ..end(() async {
        log.add('end');
        await endRelease.future;
      })
      // Child onEndHold + parent disposeEnd during unmount.
      ..end(() async {
        log.add('end-dup');
      });
    final disposeFuture = session.disposeEnd(() async {
      log.add('dispose-end');
    });
    endRelease.complete();
    await disposeFuture;
    await session.drain();

    expect(log, ['end']);
  });
}
