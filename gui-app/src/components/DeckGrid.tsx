import { DeckMixer } from "./DeckMixer";
import { DualDeckWaveform } from "./DualDeckWaveform";
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable";
import { DECK_ACCENTS, DECK_LABELS } from "../lib/ui";
import { DEFAULT_DECK_EQ, type DeckEq, type DeckStatus } from "../types";
import type { TrackDragPayload } from "../lib/libraryTable";
import { DeckPanel } from "./DeckPanel";

/** Dual-lane scrolling waveform strip. */
const WAVEFORM_MIN_HEIGHT = "70px";
const WAVEFORM_DEFAULT_HEIGHT = "112px";
const WAVEFORM_MAX_HEIGHT = "400px";
const DECK_ROW_MIN_HEIGHT = "340px";
const DECK_ROW_DEFAULT_HEIGHT = "350px";

interface DeckGridProps {
  decks: DeckStatus[];
  engineRunning: boolean;
  busy: boolean;
  onPickTrack: (deckId: number) => void;
  onTogglePlayback: (deckId: number, playing: boolean) => void;
  onVolumeChange: (deckId: number, volume: number) => void;
  onEqChange: (deckId: number, eq: DeckEq) => void;
  onSpeedChange: (deckId: number, speed: number) => void;
  onDropTrack: (deckId: number, payload: TrackDragPayload) => void;
  onSeek: (deckId: number, positionSecs: number) => void;
  onSetCuePoint: (deckId: number) => void;
  onBeginCueHold: (deckId: number) => void;
  onEndCueHold: (deckId: number) => void;
  onTriggerHotCue: (deckId: number, slot: number) => void;
  onSaveHotCue: (deckId: number, slot: number) => void;
  onDeleteHotCue: (deckId: number, slot: number) => void;
  onAutoLoop: (deckId: number, beats: number) => void;
  onLoopIn: (deckId: number) => void;
  onLoopOut: (deckId: number) => void;
  onExitLoop: (deckId: number) => void;
  onToggleQuantize: (deckId: number, enabled: boolean) => void;
  onUnload: (deckId: number) => void;
  crossfader: number;
  onCrossfaderChange: (position: number) => void;
}

function defaultDecks(): DeckStatus[] {
  return DECK_LABELS.map((_, id) => ({
    id,
    track: null,
    track_id: null,
    title: null,
    artist: null,
    bpm: null,
    key: null,
    playing: false,
    volume: 1,
    speed: 1,
    eq: DEFAULT_DECK_EQ,
    position_secs: null,
    duration_secs: null,
    cue_point_secs: null,
    quantize: true,
    hot_cues: [],
    saved_loops: [],
    active_loop: null,
  }));
}

export function DeckGrid({
  decks,
  engineRunning,
  busy,
  onPickTrack,
  onTogglePlayback,
  onVolumeChange,
  onEqChange,
  onSpeedChange,
  onDropTrack,
  onSeek,
  onSetCuePoint,
  onBeginCueHold,
  onEndCueHold,
  onTriggerHotCue,
  onSaveHotCue,
  onDeleteHotCue,
  onAutoLoop,
  onLoopIn,
  onLoopOut,
  onExitLoop,
  onToggleQuantize,
  onUnload,
  crossfader,
  onCrossfaderChange,
}: DeckGridProps) {
  const deckList = decks.length > 0 ? decks : defaultDecks();
  const accents = [DECK_ACCENTS.a, DECK_ACCENTS.b] as const;

  return (
    <section className="flex h-full min-h-0 flex-col">
      <ResizablePanelGroup
        id="deck-waveform-split"
        orientation="vertical"
        className="min-h-0 flex-1"
      >
        <ResizablePanel
          id="waveforms"
          defaultSize={WAVEFORM_DEFAULT_HEIGHT}
          minSize={WAVEFORM_MIN_HEIGHT}
          maxSize={WAVEFORM_MAX_HEIGHT}
          className="min-h-0 overflow-hidden"
        >
          <DualDeckWaveform decks={deckList} />
        </ResizablePanel>

        <ResizableHandle
          withHandle
          className="bg-white/8 hover:bg-emerald-500/25"
        />

        <ResizablePanel
          id="decks"
          defaultSize={DECK_ROW_DEFAULT_HEIGHT}
          minSize={DECK_ROW_MIN_HEIGHT}
          groupResizeBehavior="preserve-pixel-size"
          className="min-h-[340px] overflow-hidden"
        >
          <div className="grid h-full min-h-0 grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)]">
        <div className="col-start-1 min-h-0 min-w-0 overflow-hidden">
          <DeckPanel
            accent={accents[0]}
            accentKey="a"
            deck={deckList[0]}
            engineRunning={engineRunning}
            busy={busy}
            onPickTrack={() => onPickTrack(deckList[0].id)}
            onTogglePlayback={() =>
              onTogglePlayback(deckList[0].id, deckList[0].playing)
            }
            onSpeedChange={(speed) => onSpeedChange(deckList[0].id, speed)}
            onSeek={(positionSecs) => onSeek(deckList[0].id, positionSecs)}
            onSetCuePoint={() => onSetCuePoint(deckList[0].id)}
            onBeginCueHold={() => onBeginCueHold(deckList[0].id)}
            onEndCueHold={() => onEndCueHold(deckList[0].id)}
            onTriggerHotCue={(slot) => onTriggerHotCue(deckList[0].id, slot)}
            onSaveHotCue={(slot) => onSaveHotCue(deckList[0].id, slot)}
            onDeleteHotCue={(slot) => onDeleteHotCue(deckList[0].id, slot)}
            onAutoLoop={(beats) => onAutoLoop(deckList[0].id, beats)}
            onLoopIn={() => onLoopIn(deckList[0].id)}
            onLoopOut={() => onLoopOut(deckList[0].id)}
            onExitLoop={() => onExitLoop(deckList[0].id)}
            onToggleQuantize={(enabled) =>
              onToggleQuantize(deckList[0].id, enabled)
            }
            onUnload={() => onUnload(deckList[0].id)}
            onDropTrack={(payload) => onDropTrack(deckList[0].id, payload)}
          />
        </div>

        <div className="col-start-2 min-h-0 shrink-0 overflow-hidden">
          <DeckMixer
            decks={deckList}
            crossfader={crossfader}
            disabled={busy}
            onVolumeChange={onVolumeChange}
            onEqChange={onEqChange}
            onCrossfaderChange={onCrossfaderChange}
          />
        </div>

        <div className="col-start-3 min-h-0 min-w-0 overflow-hidden">
          <DeckPanel
            accent={accents[1]}
            accentKey="b"
            deck={deckList[1]}
            engineRunning={engineRunning}
            busy={busy}
            onPickTrack={() => onPickTrack(deckList[1].id)}
            onTogglePlayback={() =>
              onTogglePlayback(deckList[1].id, deckList[1].playing)
            }
            onSpeedChange={(speed) => onSpeedChange(deckList[1].id, speed)}
            onSeek={(positionSecs) => onSeek(deckList[1].id, positionSecs)}
            onSetCuePoint={() => onSetCuePoint(deckList[1].id)}
            onBeginCueHold={() => onBeginCueHold(deckList[1].id)}
            onEndCueHold={() => onEndCueHold(deckList[1].id)}
            onTriggerHotCue={(slot) => onTriggerHotCue(deckList[1].id, slot)}
            onSaveHotCue={(slot) => onSaveHotCue(deckList[1].id, slot)}
            onDeleteHotCue={(slot) => onDeleteHotCue(deckList[1].id, slot)}
            onAutoLoop={(beats) => onAutoLoop(deckList[1].id, beats)}
            onLoopIn={() => onLoopIn(deckList[1].id)}
            onLoopOut={() => onLoopOut(deckList[1].id)}
            onExitLoop={() => onExitLoop(deckList[1].id)}
            onToggleQuantize={(enabled) =>
              onToggleQuantize(deckList[1].id, enabled)
            }
            onUnload={() => onUnload(deckList[1].id)}
            onDropTrack={(payload) => onDropTrack(deckList[1].id, payload)}
          />
        </div>
          </div>
        </ResizablePanel>
      </ResizablePanelGroup>
    </section>
  );
}
