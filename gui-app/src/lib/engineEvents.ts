import type { DeckStatus, EngineStatus } from "../types";

export const ENGINE_EVENT = "engine://event";

export type EngineEvent =
  | { type: "status"; revision: number; status: EngineStatus }
  | { type: "deck_updated"; revision: number; deck: DeckStatus }
  | { type: "notice"; message: string }
  | { type: "error"; message: string };

export function applyEngineEvent(
  current: EngineStatus | null,
  event: EngineEvent,
  lastRevision: number,
): { status: EngineStatus | null; revision: number } {
  if (event.type === "status") {
    if (event.revision < lastRevision) {
      return { status: current, revision: lastRevision };
    }
    return { status: event.status, revision: event.revision };
  }

  if (event.type === "deck_updated") {
    if (event.revision < lastRevision) {
      return { status: current, revision: lastRevision };
    }
    if (!current) {
      return { status: null, revision: event.revision };
    }
    return {
      status: {
        ...current,
        decks: current.decks.map((deck) =>
          deck.id === event.deck.id ? event.deck : deck,
        ),
      },
      revision: event.revision,
    };
  }

  return { status: current, revision: lastRevision };
}

export function patchDeckPosition(
  status: EngineStatus,
  deckId: number,
  positionSecs: number,
): EngineStatus {
  return {
    ...status,
    decks: status.decks.map((deck) =>
      deck.id === deckId ? { ...deck, position_secs: positionSecs } : deck,
    ),
  };
}
