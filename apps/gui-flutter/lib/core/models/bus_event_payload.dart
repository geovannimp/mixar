import 'package:gui_flutter/src/rust/api/event_mirrors.dart';

/// Typed access to the payload a bus event carries.
///
/// The generated union wraps each payload positionally (`field0`), so these
/// getters are what UI code reads instead of matching every call site.
extension EventPayload on Event {
  DeckUpdated? get deckUpdated => switch (this) {
    Event_DeckUpdated(:final field0) => field0,
    _ => null,
  };

  DeckPosition? get deckPosition => switch (this) {
    Event_DeckPosition(:final field0) => field0,
    _ => null,
  };

  DeckLevels? get deckLevels => switch (this) {
    Event_DeckLevels(:final field0) => field0,
    _ => null,
  };

  EngineSnapshot? get engineSnapshot => switch (this) {
    Event_EngineSnapshot(:final field0) => field0,
    _ => null,
  };

  Error? get busError => switch (this) {
    Event_Error(:final field0) => field0,
    _ => null,
  };

  Notice? get busNotice => switch (this) {
    Event_Notice(:final field0) => field0,
    _ => null,
  };
}
