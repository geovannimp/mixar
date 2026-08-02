import { open } from "@tauri-apps/plugin-dialog";
import { create } from "zustand";
import { toastManager } from "@/components/ui/toast";
import { getSupportedAudioExtensions } from "@/lib/audio-extensions";
import { applyBusEvent } from "@/lib/engine/apply-bus-event";
import { getEngineTransport } from "@/lib/engine/transport";
import { getDeckOrigin, type CmdKind, type Origin } from "@/lib/engine/wire";
import { getLibraryTransport } from "@/lib/library/transport";
import { patchDeckPosition } from "@/lib/engine-events";
import { cyclePadMode } from "@/lib/pad-modes";
import {
  ZERO_DECK_LEVELS,
  type DeckEq,
  type DeckHotCueMarker,
  type DeckSavedLoop,
  type DeckStatus,
  type EngineStatus,
  type JogMode,
  type LevelMeterMode,
  type PadMode,
  type SamplerPlayMode,
} from "@/types";
import { getDefaultDeck } from "./default-deck";
const ENGINE_ERROR_TOAST_ID = "engine-error";

function reportEngineError(message: string) {
  toastManager.add({
    id: ENGINE_ERROR_TOAST_ID,
    title: message,
    type: "error",
  });
}

async function publishCmd(
  origin: Origin,
  kind: CmdKind,
  fields: Record<string, unknown> = {},
): Promise<void> {
  try {
    await getEngineTransport().publish(origin, kind, {
      ...fields,
      action_timestamp_ms: Date.now(),
    });
  } catch (err) {
    reportEngineError(String(err));
  }
}

let busUnlisten: (() => void) | null = null;
let busSubscribePromise: Promise<void> | null = null;

async function ensureBusSubscribed(): Promise<void> {
  if (busUnlisten) {
    return;
  }
  if (!busSubscribePromise) {
    busSubscribePromise = (async () => {
      busUnlisten = await getEngineTransport().subscribe((bytes) => {
        useEngineStore.getState().applyBusBytes(bytes);
      });
    })();
  }
  await busSubscribePromise;
}

/** Test helper: clear bus subscribe state between tests. */
export function resetEngineBusSubscriptionForTests(): void {
  busUnlisten?.();
  busUnlisten = null;
  busSubscribePromise = null;
}

async function publishLibraryCmd(
  kind: "delete_hot_cue" | "save_loop" | "delete_loop",
  fields: Record<string, unknown>,
): Promise<void> {
  try {
    await getLibraryTransport().publish("library", kind, fields);
  } catch (err) {
    reportEngineError(String(err));
  }
}

function getDeck(status: EngineStatus | null, deckId: number): DeckStatus {
  const defaults = getDefaultDeck(deckId);
  const deck = status?.decks[deckId];
  if (!deck) {
    return defaults;
  }
  return {
    ...defaults,
    ...deck,
    levels: deck.levels ?? ZERO_DECK_LEVELS,
  };
}

interface EngineStoreState {
  status: EngineStatus | null;
  busyDecks: [boolean, boolean];
  revision: number;
  starting: boolean;
  levelMeterMode: LevelMeterMode;
  applyBusBytes: (bytes: Uint8Array) => void;
  setLevelMeterMode: (mode: LevelMeterMode) => void;
  runDeckBlockingAction: (deckId: number, action: () => Promise<void>) => Promise<void>;
  ensureEngineRunning: () => Promise<void>;
  loadLibraryTrackToDeck: (deckId: number, trackId: string) => Promise<void>;
  loadPathToDeck: (deckId: number, path: string) => Promise<void>;
  pickTrack: (deckId: number) => Promise<void>;
  playDeck: (deckId: number) => Promise<void>;
  pauseDeck: (deckId: number) => Promise<void>;
  setDeckVolume: (deckId: number, volume: number) => Promise<void>;
  setDeckEq: (deckId: number, eq: DeckEq) => Promise<void>;
  setDeckSpeed: (deckId: number, speed: number) => Promise<void>;
  setCrossfader: (position: number) => Promise<void>;
  setCueMix: (mix: number) => Promise<void>;
  setMasterCue: (enabled: boolean) => Promise<void>;
  seekDeck: (deckId: number, positionMs: number) => Promise<void>;
  jogTouch: (deckId: number, touching: boolean) => Promise<void>;
  jogTurn: (deckId: number, delta: number) => Promise<void>;
  setJogMode: (deckId: number, top: JogMode, outer: JogMode) => Promise<void>;
  unloadDeck: (deckId: number) => Promise<void>;
  setDeckCuePoint: (deckId: number) => Promise<void>;
  beginDeckCueHold: (deckId: number) => Promise<void>;
  endDeckCueHold: (deckId: number) => Promise<void>;
  setDeckQuantize: (deckId: number, enabled: boolean) => Promise<void>;
  setDeckAutoLoop: (deckId: number, beats: number) => Promise<void>;
  setDeckLoopIn: (deckId: number) => Promise<void>;
  setDeckLoopOut: (deckId: number) => Promise<void>;
  exitDeckLoop: (deckId: number) => Promise<void>;
  triggerHotCue: (deckId: number, cue: DeckHotCueMarker) => Promise<void>;
  saveHotCue: (deckId: number, slot: number) => Promise<void>;
  deleteHotCue: (trackId: string, slot: number) => Promise<void>;
  saveLoop: (deckId: number, slot: number) => Promise<void>;
  triggerLoop: (deckId: number, loop: DeckSavedLoop) => Promise<void>;
  deleteLoop: (trackId: string, slot: number) => Promise<void>;
  toggleDeckSync: (deckId: number, beatSync?: boolean) => Promise<void>;
  setMasterDeck: (deckId: number) => Promise<void>;
  beatJumpDeck: (deckId: number, beats: number) => Promise<void>;
  cycleDeckPadMode: (deckId: number, direction: number) => Promise<void>;
  setDeckPadMode: (deckId: number, mode: PadMode) => Promise<void>;
  setDeckFilter: (deckId: number, filterDb: number) => Promise<void>;
  setDeckGainTrim: (deckId: number, gainDb: number) => Promise<void>;
  setDeckHeadphoneCue: (deckId: number, enabled: boolean) => Promise<void>;
  beginLoopRoll: (deckId: number, beats: number) => Promise<void>;
  endLoopRoll: (deckId: number) => Promise<void>;
  triggerSamplerPad: (deckId: number, slot: number) => Promise<void>;
  endSamplerPad: (deckId: number, slot: number) => Promise<void>;
  assignSamplerFromTrack: (slot: number, trackId: string, deckId?: number) => Promise<void>;
  assignSamplerFromPath: (slot: number, path: string, deckId?: number) => Promise<void>;
  clearSamplerSlot: (slot: number, deckId?: number) => Promise<void>;
  setDeckSamplerBank: (deckId: number, bankId: string) => Promise<void>;
  updateSamplerBank: (
    bankId: string,
    name: string,
    playMode: SamplerPlayMode | null,
  ) => Promise<void>;
  createSamplerBank: (deckId?: number, name?: string) => Promise<void>;
}

export const useEngineStore = create<EngineStoreState>((set, get) => ({
  status: null,
  busyDecks: [false, false],
  revision: 0,
  starting: false,
  levelMeterMode: "mono",

  applyBusBytes: (bytes) => {
    set((current) => {
      try {
        const patch = applyBusEvent(current.status, current.revision, bytes);
        if (patch.error) {
          reportEngineError(patch.error);
        }
        if (patch.notice) {
          toastManager.add({ title: patch.notice, type: "info" });
        }
        return { status: patch.status, revision: patch.revision };
      } catch (err) {
        console.error("engine bus event decode failed", err);
        return current;
      }
    });
  },

  setLevelMeterMode: (mode) => set({ levelMeterMode: mode }),

  runDeckBlockingAction: async (deckId, action) => {
    if (deckId < 0 || deckId > 1) {
      return;
    }
    set((state) => {
      const busyDecks = [...state.busyDecks] as [boolean, boolean];
      busyDecks[deckId] = true;
      return { busyDecks };
    });
    try {
      await action();
    } catch (err) {
      reportEngineError(String(err));
    } finally {
      set((state) => {
        const busyDecks = [...state.busyDecks] as [boolean, boolean];
        busyDecks[deckId] = false;
        return { busyDecks };
      });
    }
  },

  ensureEngineRunning: async () => {
    await ensureBusSubscribed();
    const { status, starting } = get();
    if (status?.running || starting) {
      return;
    }

    set({ starting: true });
    try {
      await toastManager.promise(
        (async () => {
          await getEngineTransport().publish("engine", "start_engine", {
            action_timestamp_ms: Date.now(),
          });
        })(),
        {
          loading: { title: "Starting engine…", type: "loading" },
          success: { title: "Engine running", type: "success" },
          error: (err: unknown) => ({
            title: err instanceof Error ? err.message : String(err),
            type: "error",
          }),
        },
      );
    } finally {
      set({ starting: false });
    }
  },

  loadPathToDeck: async (deckId, path) => {
    await get().runDeckBlockingAction(deckId, async () => {
      await publishCmd(getDeckOrigin(deckId), "load_path", { path });
    });
  },

  pickTrack: async (deckId) => {
    const extensions = await getSupportedAudioExtensions();
    const selected = await open({
      multiple: false,
      filters: [{ name: "Audio", extensions }],
    });
    if (typeof selected === "string") {
      await get().loadPathToDeck(deckId, selected);
    }
  },

  loadLibraryTrackToDeck: async (deckId, trackId) => {
    await get().runDeckBlockingAction(deckId, async () => {
      await publishCmd(getDeckOrigin(deckId), "load_library_track", { track_id: trackId });
    });
  },

  playDeck: async (deckId) => {
    await publishCmd(getDeckOrigin(deckId), "play");
  },

  pauseDeck: async (deckId) => {
    await publishCmd(getDeckOrigin(deckId), "pause");
  },

  setDeckVolume: async (deckId, volume) => {
    await publishCmd(getDeckOrigin(deckId), "set_volume", { volume });
  },

  setDeckEq: async (deckId, eq) => {
    await publishCmd(getDeckOrigin(deckId), "set_eq", {
      low: eq.low,
      mid: eq.mid,
      high: eq.high,
    });
  },

  setDeckSpeed: async (deckId, speed) => {
    await publishCmd(getDeckOrigin(deckId), "set_speed", { speed });
  },

  setCrossfader: async (position) => {
    await publishCmd("mixer", "set_crossfader", { position });
  },

  setCueMix: async (mix) => {
    await publishCmd("mixer", "set_cue_mix", { mix });
  },

  setMasterCue: async (enabled) => {
    await publishCmd("mixer", "set_master_cue", { enabled });
  },

  seekDeck: async (deckId, positionMs) => {
    const clamped = Number.isFinite(positionMs) ? Math.trunc(positionMs) : 0;
    const status = get().status;
    if (status) {
      set({ status: patchDeckPosition(status, deckId, clamped) });
    }
    await publishCmd(getDeckOrigin(deckId), "seek", {
      position_ms: clamped,
    });
  },

  jogTouch: async (deckId, touching) => {
    await publishCmd(getDeckOrigin(deckId), "jog_touch", { touching });
  },

  jogTurn: async (deckId, delta) => {
    if (!Number.isFinite(delta) || delta === 0) {
      return;
    }
    await publishCmd(getDeckOrigin(deckId), "jog_turn", {
      delta: Math.trunc(delta),
    });
  },

  setJogMode: async (deckId, top, outer) => {
    await publishCmd(getDeckOrigin(deckId), "set_jog_mode", { top, outer });
  },

  unloadDeck: async (deckId) => {
    await get().runDeckBlockingAction(deckId, async () => {
      await publishCmd(getDeckOrigin(deckId), "unload");
    });
  },

  setDeckCuePoint: async (deckId) => {
    await publishCmd(getDeckOrigin(deckId), "set_cue_point");
  },

  beginDeckCueHold: async (deckId) => {
    await publishCmd(getDeckOrigin(deckId), "begin_cue_hold");
  },

  endDeckCueHold: async (deckId) => {
    await publishCmd(getDeckOrigin(deckId), "end_cue_hold");
  },

  setDeckQuantize: async (deckId, enabled) => {
    await publishCmd(getDeckOrigin(deckId), "set_quantize", { enabled });
  },

  setDeckAutoLoop: async (deckId, beats) => {
    await publishCmd(getDeckOrigin(deckId), "set_auto_loop", { beats });
  },

  setDeckLoopIn: async (deckId) => {
    await publishCmd(getDeckOrigin(deckId), "loop_in");
  },

  setDeckLoopOut: async (deckId) => {
    await publishCmd(getDeckOrigin(deckId), "loop_out");
  },

  exitDeckLoop: async (deckId) => {
    await publishCmd(getDeckOrigin(deckId), "exit_loop");
  },

  triggerHotCue: async (deckId, cue) => {
    await publishCmd(getDeckOrigin(deckId), "trigger_hot_cue", {
      position_ms: cue.position_ms,
    });
  },

  saveHotCue: async (deckId, slot) => {
    await publishCmd(getDeckOrigin(deckId), "save_hot_cue", { slot });
  },

  deleteHotCue: async (trackId, slot) => {
    await publishLibraryCmd("delete_hot_cue", { track_id: trackId, slot });
  },

  saveLoop: async (deckId, slot) => {
    const deck = getDeck(get().status, deckId);
    const region = deck.active_loop;
    await publishLibraryCmd("save_loop", {
      track_id: deck.track_id,
      slot,
      in_ms: region?.in_ms,
      out_ms: region?.out_ms,
    });
  },

  triggerLoop: async (deckId, loop) => {
    await publishCmd(getDeckOrigin(deckId), "recall_saved_loop", {
      in_ms: loop.in_ms,
      out_ms: loop.out_ms,
    });
  },

  deleteLoop: async (trackId, slot) => {
    await publishLibraryCmd("delete_loop", { track_id: trackId, slot });
  },

  toggleDeckSync: async (deckId, beatSync = false) => {
    await publishCmd(getDeckOrigin(deckId), "toggle_sync", {
      beat_sync: beatSync,
    });
  },

  setMasterDeck: async (deckId) => {
    await publishCmd(getDeckOrigin(deckId), "set_master_deck");
  },

  beatJumpDeck: async (deckId, beats) => {
    await publishCmd(getDeckOrigin(deckId), "beat_jump", { beats });
  },

  cycleDeckPadMode: async (deckId, direction) => {
    const current = getDeck(get().status, deckId).pad_mode;
    await get().setDeckPadMode(deckId, cyclePadMode(current, direction));
  },

  setDeckPadMode: async (deckId, mode) => {
    await publishCmd(getDeckOrigin(deckId), "set_pad_mode", { mode });
  },

  setDeckFilter: async (deckId, filterDb) => {
    await publishCmd(getDeckOrigin(deckId), "set_filter", {
      filter_db: filterDb,
    });
  },

  setDeckGainTrim: async (deckId, gainDb) => {
    await publishCmd(getDeckOrigin(deckId), "set_gain_trim", {
      gain_db: gainDb,
    });
  },

  setDeckHeadphoneCue: async (deckId, enabled) => {
    await publishCmd(getDeckOrigin(deckId), "set_headphone_cue", { enabled });
  },

  beginLoopRoll: async (deckId, beats) => {
    await publishCmd(getDeckOrigin(deckId), "begin_loop_roll", { beats });
  },

  endLoopRoll: async (deckId) => {
    await publishCmd(getDeckOrigin(deckId), "end_loop_roll");
  },

  triggerSamplerPad: async (deckId, slot) => {
    await publishCmd(getDeckOrigin(deckId), "trigger_sampler", { slot });
  },

  endSamplerPad: async (deckId, slot) => {
    await publishCmd(getDeckOrigin(deckId), "end_sampler", { slot });
  },

  assignSamplerFromTrack: async (slot, trackId, deckId = 0) => {
    await publishCmd(getDeckOrigin(deckId), "assign_sampler_track", {
      slot,
      track_id: trackId,
    });
  },

  assignSamplerFromPath: async (slot, path, deckId = 0) => {
    await publishCmd(getDeckOrigin(deckId), "assign_sampler", { slot, path });
  },

  clearSamplerSlot: async (slot, deckId = 0) => {
    await publishCmd(getDeckOrigin(deckId), "clear_sampler", { slot });
  },

  setDeckSamplerBank: async (deckId, bankId) => {
    await publishCmd(getDeckOrigin(deckId), "set_sampler_bank", { bank_id: bankId });
  },

  updateSamplerBank: async (bankId, name, playMode) => {
    await publishCmd("mixer", "update_sampler_bank", {
      bank_id: bankId,
      name,
      play_mode: playMode,
    });
  },

  createSamplerBank: async (deckId = 0, name) => {
    await publishCmd(getDeckOrigin(deckId), "create_sampler_bank", {
      name: name?.trim() || null,
      play_mode: null,
    });
  },
}));

export const engineActions = {
  ensureEngineRunning: () => useEngineStore.getState().ensureEngineRunning(),
  setLevelMeterMode: (mode: LevelMeterMode) => useEngineStore.getState().setLevelMeterMode(mode),
  loadLibraryTrackToDeck: (deckId: number, trackId: string) =>
    useEngineStore.getState().loadLibraryTrackToDeck(deckId, trackId),
  loadPathToDeck: (deckId: number, path: string) =>
    useEngineStore.getState().loadPathToDeck(deckId, path),
  pickTrack: (deckId: number) => useEngineStore.getState().pickTrack(deckId),
  playDeck: (deckId: number) => useEngineStore.getState().playDeck(deckId),
  pauseDeck: (deckId: number) => useEngineStore.getState().pauseDeck(deckId),
  setDeckVolume: (deckId: number, volume: number) =>
    useEngineStore.getState().setDeckVolume(deckId, volume),
  setDeckEq: (deckId: number, eq: DeckEq) => useEngineStore.getState().setDeckEq(deckId, eq),
  setDeckSpeed: (deckId: number, speed: number) =>
    useEngineStore.getState().setDeckSpeed(deckId, speed),
  setCrossfader: (position: number) => useEngineStore.getState().setCrossfader(position),
  setCueMix: (mix: number) => useEngineStore.getState().setCueMix(mix),
  setMasterCue: (enabled: boolean) => useEngineStore.getState().setMasterCue(enabled),
  seekDeck: (deckId: number, positionMs: number) =>
    useEngineStore.getState().seekDeck(deckId, positionMs),
  jogTouch: (deckId: number, touching: boolean) =>
    useEngineStore.getState().jogTouch(deckId, touching),
  jogTurn: (deckId: number, delta: number) => useEngineStore.getState().jogTurn(deckId, delta),
  setJogMode: (deckId: number, top: JogMode, outer: JogMode) =>
    useEngineStore.getState().setJogMode(deckId, top, outer),
  unloadDeck: (deckId: number) => useEngineStore.getState().unloadDeck(deckId),
  setDeckCuePoint: (deckId: number) => useEngineStore.getState().setDeckCuePoint(deckId),
  beginDeckCueHold: (deckId: number) => useEngineStore.getState().beginDeckCueHold(deckId),
  endDeckCueHold: (deckId: number) => useEngineStore.getState().endDeckCueHold(deckId),
  setDeckQuantize: (deckId: number, enabled: boolean) =>
    useEngineStore.getState().setDeckQuantize(deckId, enabled),
  setDeckAutoLoop: (deckId: number, beats: number) =>
    useEngineStore.getState().setDeckAutoLoop(deckId, beats),
  setDeckLoopIn: (deckId: number) => useEngineStore.getState().setDeckLoopIn(deckId),
  setDeckLoopOut: (deckId: number) => useEngineStore.getState().setDeckLoopOut(deckId),
  exitDeckLoop: (deckId: number) => useEngineStore.getState().exitDeckLoop(deckId),
  triggerHotCue: (deckId: number, cue: DeckHotCueMarker) =>
    useEngineStore.getState().triggerHotCue(deckId, cue),
  saveHotCue: (deckId: number, slot: number) => useEngineStore.getState().saveHotCue(deckId, slot),
  deleteHotCue: (trackId: string, slot: number) =>
    useEngineStore.getState().deleteHotCue(trackId, slot),
  saveLoop: (deckId: number, slot: number) => useEngineStore.getState().saveLoop(deckId, slot),
  triggerLoop: (deckId: number, loop: DeckSavedLoop) =>
    useEngineStore.getState().triggerLoop(deckId, loop),
  deleteLoop: (trackId: string, slot: number) =>
    useEngineStore.getState().deleteLoop(trackId, slot),
  toggleDeckSync: (deckId: number, beatSync?: boolean) =>
    useEngineStore.getState().toggleDeckSync(deckId, beatSync),
  setMasterDeck: (deckId: number) => useEngineStore.getState().setMasterDeck(deckId),
  beatJumpDeck: (deckId: number, beats: number) =>
    useEngineStore.getState().beatJumpDeck(deckId, beats),
  cycleDeckPadMode: (deckId: number, direction: number) =>
    useEngineStore.getState().cycleDeckPadMode(deckId, direction),
  setDeckPadMode: (deckId: number, mode: PadMode) =>
    useEngineStore.getState().setDeckPadMode(deckId, mode),
  setDeckFilter: (deckId: number, filterDb: number) =>
    useEngineStore.getState().setDeckFilter(deckId, filterDb),
  setDeckGainTrim: (deckId: number, gainDb: number) =>
    useEngineStore.getState().setDeckGainTrim(deckId, gainDb),
  setDeckHeadphoneCue: (deckId: number, enabled: boolean) =>
    useEngineStore.getState().setDeckHeadphoneCue(deckId, enabled),
  beginLoopRoll: (deckId: number, beats: number) =>
    useEngineStore.getState().beginLoopRoll(deckId, beats),
  endLoopRoll: (deckId: number) => useEngineStore.getState().endLoopRoll(deckId),
  triggerSamplerPad: (deckId: number, slot: number) =>
    useEngineStore.getState().triggerSamplerPad(deckId, slot),
  endSamplerPad: (deckId: number, slot: number) =>
    useEngineStore.getState().endSamplerPad(deckId, slot),
  assignSamplerFromTrack: (slot: number, trackId: string, deckId?: number) =>
    useEngineStore.getState().assignSamplerFromTrack(slot, trackId, deckId),
  assignSamplerFromPath: (slot: number, path: string, deckId?: number) =>
    useEngineStore.getState().assignSamplerFromPath(slot, path, deckId),
  clearSamplerSlot: (slot: number, deckId?: number) =>
    useEngineStore.getState().clearSamplerSlot(slot, deckId),
  setDeckSamplerBank: (deckId: number, bankId: string) =>
    useEngineStore.getState().setDeckSamplerBank(deckId, bankId),
  updateSamplerBank: (bankId: string, name: string, playMode: SamplerPlayMode | null) =>
    useEngineStore.getState().updateSamplerBank(bankId, name, playMode),
  createSamplerBank: (deckId?: number, name?: string) =>
    useEngineStore.getState().createSamplerBank(deckId, name),
};
