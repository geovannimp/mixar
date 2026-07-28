import { ZERO_DECK_LEVELS, type DeckLevels, type DeckStatus, type EngineStatus } from "@/types";

export const ENGINE_EVENT = "engine://event";

export type EngineEvent =
  | { type: "status"; revision: number; status: EngineStatus }
  | { type: "deck_updated"; revision: number; deck: DeckStatus };

export function applyEngineEvent(
  current: EngineStatus | null,
  event: EngineEvent,
  lastRevision: number,
): { status: EngineStatus | null; revision: number } {
  switch (event.type) {
    case "status": {
      if (event.revision < lastRevision) {
        return { status: current, revision: lastRevision };
      }
      if (!current) {
        return { status: event.status, revision: event.revision };
      }
      const currentById = new Map(current.decks.map((deck) => [deck.id, deck]));
      return {
        status: {
          ...event.status,
          decks: event.status.decks.map((deck) => ({
            ...deck,
            levels: currentById.get(deck.id)?.levels ?? ZERO_DECK_LEVELS,
          })),
        },
        revision: event.revision,
      };
    }
    case "deck_updated": {
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
            deck.id === event.deck.id
              ? { ...event.deck, levels: deck.levels ?? ZERO_DECK_LEVELS }
              : deck,
          ),
        },
        revision: event.revision,
      };
    }
    default: {
      const _exhaustive: never = event;
      void _exhaustive;
      return { status: current, revision: lastRevision };
    }
  }
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

export function patchDeckLevels(
  status: EngineStatus,
  deckId: number,
  levels: DeckLevels,
): EngineStatus {
  return {
    ...status,
    decks: status.decks.map((deck) => (deck.id === deckId ? { ...deck, levels } : deck)),
  };
}
