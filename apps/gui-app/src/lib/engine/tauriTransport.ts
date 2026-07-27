import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { EngineTransport } from "@/lib/engine/transport";
import {
  encodePause,
  encodePlay,
  encodeSeek,
  encodeSetCrossfader,
  encodeSetCueMix,
  encodeSetEq,
  encodeSetFilter,
  encodeSetGainTrim,
  encodeSetHeadphoneCue,
  encodeSetMasterCue,
  encodeSetVolume,
} from "@/lib/engine/wire";

export const ENGINE_BUS_EVENT = "engine://bus";

function publishWire(bytes: Uint8Array): Promise<void> {
  return invoke("engine_publish", {
    payload: Array.from(bytes),
  });
}

export function createTauriEngineTransport(): EngineTransport {
  return {
    play: (deckId) => publishWire(encodePlay(deckId)),
    pause: (deckId) => publishWire(encodePause(deckId)),
    seek: (deckId, positionSecs) => publishWire(encodeSeek(deckId, positionSecs)),
    setVolume: (deckId, volume) => publishWire(encodeSetVolume(deckId, volume)),
    setEq: (deckId, low, mid, high) => publishWire(encodeSetEq(deckId, low, mid, high)),
    setFilter: (deckId, filterDb) => publishWire(encodeSetFilter(deckId, filterDb)),
    setGainTrim: (deckId, gainDb) => publishWire(encodeSetGainTrim(deckId, gainDb)),
    setHeadphoneCue: (deckId, enabled) => publishWire(encodeSetHeadphoneCue(deckId, enabled)),
    setCrossfader: (position) => publishWire(encodeSetCrossfader(position)),
    setCueMix: (mix) => publishWire(encodeSetCueMix(mix)),
    setMasterCue: (enabled) => publishWire(encodeSetMasterCue(enabled)),
    subscribe: (handler) => {
      const unlistenPromise = listen<number[] | Uint8Array>(ENGINE_BUS_EVENT, (event) => {
        const payload = event.payload;
        const bytes = payload instanceof Uint8Array ? payload : Uint8Array.from(payload ?? []);
        if (bytes.length === 0) {
          return;
        }
        handler(bytes);
      });
      return () => {
        void unlistenPromise.then((unlisten) => unlisten());
      };
    },
  };
}
