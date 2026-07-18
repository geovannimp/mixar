import { ZERO_DECK_LEVELS, type DeckLevels, type DeckStatus, type EngineStatus } from "@/types";

export const ENGINE_EVENT = "engine://event";

export type EngineEvent =
  | { type: "status"; revision: number; status: EngineStatus }
  | { type: "deck_updated"; revision: number; deck: DeckStatus }
  | { type: "position"; deck_id: number; position_secs: number }
  | {
      type: "levels";
      deck_id: number;
      peak_l: number;
      peak_r: number;
      peak_hold_l: number;
      peak_hold_r: number;
    }
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
          deck.id === event.deck.id
            ? { ...event.deck, levels: deck.levels ?? ZERO_DECK_LEVELS }
            : deck,
        ),
      },
      revision: event.revision,
    };
  }

  if (event.type === "position") {
    if (!current) {
      return { status: current, revision: lastRevision };
    }
    return {
      status: patchDeckPosition(current, event.deck_id, event.position_secs),
      revision: lastRevision,
    };
  }

  if (event.type === "levels") {
    if (!current) {
      return { status: current, revision: lastRevision };
    }
    const levels: DeckLevels = {
      peak_l: event.peak_l,
      peak_r: event.peak_r,
      peak_hold_l: event.peak_hold_l,
      peak_hold_r: event.peak_hold_r,
    };
    return {
      status: patchDeckLevels(current, event.deck_id, levels),
      revision: lastRevision,
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
