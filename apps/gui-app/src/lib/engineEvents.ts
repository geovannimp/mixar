import type { DeckLevels, EngineStatus } from "@/types";

export function patchDeckPosition(
  status: EngineStatus,
  deckId: number,
  positionMs: number,
): EngineStatus {
  return {
    ...status,
    decks: status.decks.map((deck) =>
      deck.id === deckId ? { ...deck, position_ms: positionMs } : deck,
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
