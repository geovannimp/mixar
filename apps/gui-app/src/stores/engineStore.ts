import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { create } from "zustand";
import { useShallow } from "zustand/react/shallow";
import { toastManager } from "@/components/ui/toast";
import { getSupportedAudioExtensions } from "@/lib/audioExtensions";
import { applyBusEvent } from "@/lib/engine/applyBusEvent";
import { getEngineTransport } from "@/lib/engine/transport";
import { getDeckOrigin, type CmdKind, type Origin } from "@/lib/engine/wire";
import { patchDeckPosition } from "@/lib/engineEvents";
import { cyclePadMode } from "@/lib/padModes";
import {
  ZERO_DECK_LEVELS,
  type DeckEq,
  type DeckStatus,
  type EngineStatus,
  type LevelMeterMode,
  type PadMode,
  type SamplerBankInfo,
  type SamplerPlayMode,
  type SamplerSlotInfo,
  type SamplerStatus,
} from "@/types";
import { getDefaultDeck } from "./defaultDeck";
import { DEFAULT_SAMPLER_STATUS, EMPTY_SAMPLER_BANKS, EMPTY_SAMPLER_SLOTS } from "./defaultSampler";
const ENGINE_ERROR_TOAST_ID = "engine-error";
const engineTransport = getEngineTransport();

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
    await engineTransport.publish(origin, kind, {
      ...fields,
      action_timestamp_ms: Date.now(),
    });
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
  setStatus: (status: EngineStatus | null) => void;
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
  seekDeck: (deckId: number, positionSecs: number) => Promise<void>;
  unloadDeck: (deckId: number) => Promise<void>;
  setDeckCuePoint: (deckId: number) => Promise<void>;
  beginDeckCueHold: (deckId: number) => Promise<void>;
  endDeckCueHold: (deckId: number) => Promise<void>;
  setDeckQuantize: (deckId: number, enabled: boolean) => Promise<void>;
  setDeckAutoLoop: (deckId: number, beats: number) => Promise<void>;
  setDeckLoopIn: (deckId: number) => Promise<void>;
  setDeckLoopOut: (deckId: number) => Promise<void>;
  exitDeckLoop: (deckId: number) => Promise<void>;
  triggerHotCue: (deckId: number, slot: number) => Promise<void>;
  saveHotCue: (deckId: number, slot: number) => Promise<void>;
  deleteHotCue: (deckId: number, slot: number) => Promise<void>;
  saveLoop: (deckId: number, slot: number) => Promise<void>;
  recallSavedLoop: (deckId: number, slot: number) => Promise<void>;
  deleteLoop: (deckId: number, slot: number) => Promise<void>;
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

  setStatus: (status) => set({ status }),

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
    const { status, starting } = get();
    if (status?.running || starting) {
      return;
    }

    set({ starting: true });
    try {
      await toastManager.promise(
        (async () => {
          await invoke("start_engine");
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

  seekDeck: async (deckId, positionSecs) => {
    const status = get().status;
    if (status) {
      set({ status: patchDeckPosition(status, deckId, positionSecs) });
    }
    await publishCmd(getDeckOrigin(deckId), "seek", {
      position_secs: positionSecs,
    });
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

  triggerHotCue: async (deckId, slot) => {
    const cue = getDeck(get().status, deckId).hot_cues.find((entry) => entry.slot === slot);
    if (!cue) {
      reportEngineError(`Hot cue ${slot + 1} is empty.`);
      return;
    }
    await publishCmd(getDeckOrigin(deckId), "trigger_hot_cue", {
      position_secs: cue.position_secs,
    });
  },

  saveHotCue: async (deckId, slot) => {
    await publishCmd(getDeckOrigin(deckId), "save_hot_cue", { slot });
  },

  deleteHotCue: async (deckId, slot) => {
    await publishCmd(getDeckOrigin(deckId), "delete_hot_cue", { slot });
  },

  saveLoop: async (deckId, slot) => {
    await publishCmd(getDeckOrigin(deckId), "save_loop", { slot });
  },

  recallSavedLoop: async (deckId, slot) => {
    const saved = getDeck(get().status, deckId).saved_loops.find((entry) => entry.slot === slot);
    if (!saved) {
      reportEngineError(`Saved loop ${slot + 1} is empty.`);
      return;
    }
    await publishCmd(getDeckOrigin(deckId), "recall_saved_loop", {
      in_secs: saved.in_secs,
      out_secs: saved.out_secs,
    });
  },

  deleteLoop: async (deckId, slot) => {
    await publishCmd(getDeckOrigin(deckId), "delete_loop", { slot });
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

const selectEngineHeaderInfo = (state: EngineStoreState) => ({
  running: Boolean(state.status?.running),
  backend: state.status?.backend ?? "",
  sampleRate: state.status?.sample_rate ?? 0,
});

function selectDeckMixerChannel(state: EngineStoreState, deckId: number) {
  const deck = getDeck(state.status, deckId);
  return {
    volume: deck.volume,
    eq: deck.eq,
    filter_db: deck.filter_db,
    gain_trim_db: deck.gain_trim_db,
    headphone_cue: deck.headphone_cue,
    levels: deck.levels,
  };
}

function selectDeckTransport(state: EngineStoreState, deckId: number) {
  const deck = getDeck(state.status, deckId);
  return {
    position_secs: deck.position_secs,
    duration_secs: deck.duration_secs,
    playing: deck.playing,
  };
}

function selectDeckWaveform(state: EngineStoreState, deckId: number) {
  const deck = getDeck(state.status, deckId);
  return {
    id: deck.id,
    track: deck.track,
    track_id: deck.track_id,
    position_secs: deck.position_secs,
    playing: deck.playing,
    speed: deck.speed,
    eq: deck.eq,
    hot_cues: deck.hot_cues,
    active_loop: deck.active_loop,
    duration_secs: deck.duration_secs,
  };
}

function selectDeckControls(state: EngineStoreState, deckId: number) {
  const deck = getDeck(state.status, deckId);
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
    quantize: deck.quantize,
    cue_point_secs: deck.cue_point_secs,
    hot_cues: deck.hot_cues,
    saved_loops: deck.saved_loops,
    active_loop: deck.active_loop,
    sync_mode: deck.sync_mode,
    is_master: deck.is_master,
    pad_mode: deck.pad_mode,
    loudness_lufs: deck.loudness_lufs,
    auto_gain_db: deck.auto_gain_db,
    gain_trim_db: deck.gain_trim_db,
    active_sampler_bank_id: deck.active_sampler_bank_id,
  };
}

function selectDeckOverview(state: EngineStoreState, deckId: number) {
  const deck = getDeck(state.status, deckId);
  return {
    track_id: deck.track_id,
    track: deck.track,
    position_secs: deck.position_secs,
    playing: deck.playing,
    speed: deck.speed,
    duration_secs: deck.duration_secs,
    hot_cues: deck.hot_cues,
  };
}

const selectDeckMixerChannel0 = (state: EngineStoreState) => selectDeckMixerChannel(state, 0);
const selectDeckMixerChannel1 = (state: EngineStoreState) => selectDeckMixerChannel(state, 1);

const selectDeckTransport0 = (state: EngineStoreState) => selectDeckTransport(state, 0);
const selectDeckTransport1 = (state: EngineStoreState) => selectDeckTransport(state, 1);

const selectDeckWaveform0 = (state: EngineStoreState) => selectDeckWaveform(state, 0);
const selectDeckWaveform1 = (state: EngineStoreState) => selectDeckWaveform(state, 1);

const selectDeckControls0 = (state: EngineStoreState) => selectDeckControls(state, 0);
const selectDeckControls1 = (state: EngineStoreState) => selectDeckControls(state, 1);

const selectDeckOverview0 = (state: EngineStoreState) => selectDeckOverview(state, 0);
const selectDeckOverview1 = (state: EngineStoreState) => selectDeckOverview(state, 1);

const DECK_MIXER_CHANNEL_SELECTORS = [selectDeckMixerChannel0, selectDeckMixerChannel1] as const;

const DECK_TRANSPORT_SELECTORS = [selectDeckTransport0, selectDeckTransport1] as const;

const DECK_WAVEFORM_SELECTORS = [selectDeckWaveform0, selectDeckWaveform1] as const;

const DECK_CONTROLS_SELECTORS = [selectDeckControls0, selectDeckControls1] as const;

const DECK_OVERVIEW_SELECTORS = [selectDeckOverview0, selectDeckOverview1] as const;

function deckSelector<T>(
  deckId: number,
  selectors: readonly ((state: EngineStoreState) => T)[],
): (state: EngineStoreState) => T {
  return selectors[deckId] ?? selectors[0];
}

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
  seekDeck: (deckId: number, positionSecs: number) =>
    useEngineStore.getState().seekDeck(deckId, positionSecs),
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
  triggerHotCue: (deckId: number, slot: number) =>
    useEngineStore.getState().triggerHotCue(deckId, slot),
  saveHotCue: (deckId: number, slot: number) => useEngineStore.getState().saveHotCue(deckId, slot),
  deleteHotCue: (deckId: number, slot: number) =>
    useEngineStore.getState().deleteHotCue(deckId, slot),
  saveLoop: (deckId: number, slot: number) => useEngineStore.getState().saveLoop(deckId, slot),
  recallSavedLoop: (deckId: number, slot: number) =>
    useEngineStore.getState().recallSavedLoop(deckId, slot),
  deleteLoop: (deckId: number, slot: number) => useEngineStore.getState().deleteLoop(deckId, slot),
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

export function useEngineRunning(): boolean {
  return useEngineStore((state) => Boolean(state.status?.running));
}

export function useEngineBusy(): boolean {
  return useEngineStore((state) => state.starting || state.busyDecks[0] || state.busyDecks[1]);
}

export function useDeckBusy(deckId: number): boolean {
  return useEngineStore((state) => state.busyDecks[deckId] ?? false);
}

export function useEngineHeaderInfo() {
  return useEngineStore(useShallow(selectEngineHeaderInfo));
}

export function useCrossfader(): number {
  return useEngineStore((state) => state.status?.crossfader ?? 0.5);
}

export function useCueMix(): number {
  return useEngineStore((state) => state.status?.cue_mix ?? 0);
}

export function useMasterCue(): boolean {
  return useEngineStore((state) => state.status?.master_cue ?? false);
}

export function useLevelMeterMode(): LevelMeterMode {
  return useEngineStore((state) => state.levelMeterMode);
}

export function useDeckHasTrack(deckId: number): boolean {
  return useEngineStore((state) => Boolean(getDeck(state.status, deckId).track));
}

export function useAnyDeckHasTrack(): boolean {
  return useEngineStore((state) => Boolean(state.status?.decks.some((deck) => deck.track)));
}

export function useDeckMixerChannel(deckId: number) {
  return useEngineStore(useShallow(deckSelector(deckId, DECK_MIXER_CHANNEL_SELECTORS)));
}

export function useDeckTransport(deckId: number) {
  return useEngineStore(useShallow(deckSelector(deckId, DECK_TRANSPORT_SELECTORS)));
}

export function useDeckWaveform(deckId: number) {
  return useEngineStore(useShallow(deckSelector(deckId, DECK_WAVEFORM_SELECTORS)));
}

export function useDeckControls(deckId: number) {
  return useEngineStore(useShallow(deckSelector(deckId, DECK_CONTROLS_SELECTORS)));
}

export function useDeckOverview(deckId: number) {
  return useEngineStore(useShallow(deckSelector(deckId, DECK_OVERVIEW_SELECTORS)));
}

export function useSamplerSlots(deckId: number): SamplerSlotInfo[] {
  return useEngineStore(
    (state) => state.status?.sampler?.deck_slots[deckId] ?? EMPTY_SAMPLER_SLOTS,
  );
}

export function useSamplerStatus(): SamplerStatus {
  return useEngineStore((state) => state.status?.sampler ?? DEFAULT_SAMPLER_STATUS);
}

export function useSamplerBanks(): SamplerBankInfo[] {
  return useEngineStore((state) => state.status?.sampler?.banks ?? EMPTY_SAMPLER_BANKS);
}

export function useSamplerEffectivePlayMode(deckId: number): SamplerPlayMode {
  return useEngineStore(
    (state) => state.status?.sampler?.effective_play_modes[deckId] ?? "oneshot",
  );
}
