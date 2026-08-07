import { useShallow } from "zustand/react/shallow";
import { getDefaultDeck } from "@/stores/default-deck";
import { useEngineStore } from "@/stores/engine-store";

export function useDeckControls(deckId: number) {
  return useEngineStore(
    useShallow((state) => {
      const deck = state.status?.decks[deckId] ?? getDefaultDeck(deckId);
      return {
        id: deck.id,
        track: deck.track,
        track_id: deck.track_id,
        title: deck.title,
        artist: deck.artist,
        bpm: deck.bpm,
        key: deck.key,
        playing: deck.playing,
        speed: deck.speed,
        tempo_range: deck.tempo_range,
        quantize: deck.quantize,
        cue_point_ms: deck.cue_point_ms,
        hot_cues: deck.hot_cues,
        saved_loops: deck.saved_loops,
        active_loop: deck.active_loop,
        sync_mode: deck.sync_mode,
        is_master: deck.is_master,
        pad_mode: deck.pad_mode,
        loudness_lufs: deck.loudness_lufs,
        auto_gain_db: deck.auto_gain_db,
        gain_trim: deck.gain_trim,
        active_sampler_bank_id: deck.active_sampler_bank_id,
        top_jog_mode: deck.top_jog_mode,
        outer_jog_mode: deck.outer_jog_mode,
        jog_touching: deck.jog_touching,
      };
    }),
  );
}
