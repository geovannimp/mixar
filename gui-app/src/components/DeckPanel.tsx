import { memo, useEffect, useRef, useState } from "react";
import { Pause, Play } from "lucide-react";
import { cn } from "@/lib/utils";
import {
  acceptsTrackDrag,
  readTrackDragData,
  type TrackDragPayload,
} from "../lib/libraryTable";
import { DeckButton } from "@/components/ui/deck-button";
import { type DeckAccent, DECK_ACCENTS } from "../lib/ui";
import {
  formatDeckRemainingDisplay,
  formatDeckTotalDisplay,
} from "../lib/format";
import {
  engineActions,
  useDeckControls,
  useDeckOverview,
  useDeckTransport,
  useEngineBusy,
  useEngineRunning,
} from "../hooks/useEngine";
import { getDefaultDeck } from "../stores/defaultDeck";
import type { DeckStatus } from "../types";
import { DeckPadsPanel } from "./DeckPadsPanel";
import { DeckLoopPanel } from "./DeckLoopPanel";
import { DeckOverviewPreview } from "./DeckOverviewPreview";
import { DeckTempoPanel } from "./DeckTempoPanel";
import { DeckTrackInfo } from "./DeckTrackInfo";
import { DeckCircularButton, JogPlatter } from "./DeckTransport";

interface DeckPanelProps {
  deckId: number;
  accentKey: DeckAccent;
  focused?: boolean;
  onFocus?: () => void;
}

const DeckOverviewSection = memo(function DeckOverviewSection({
  deckId,
  transportDisabled,
}: {
  deckId: number;
  transportDisabled: boolean;
}) {
  const overview = useDeckOverview(deckId);

  return (
    <div className="flex shrink-0 flex-col gap-0.5">
      <div className="flex items-baseline justify-between gap-3 font-mono tabular-nums">
        <span className="text-sm font-semibold text-zinc-100 sm:text-base">
          {formatDeckRemainingDisplay(
            overview.position_secs,
            overview.duration_secs,
          )}
        </span>
        <span className="text-[11px] text-zinc-500 sm:text-xs">
          {formatDeckTotalDisplay(overview.duration_secs)}
        </span>
      </div>
      <DeckOverviewPreview
        trackId={overview.track_id}
        path={overview.track}
        positionSecs={overview.position_secs ?? 0}
        playing={overview.playing}
        speed={overview.speed}
        durationSecs={overview.duration_secs}
        hotCues={overview.hot_cues}
        disabled={transportDisabled}
        onSeek={(positionSecs) => {
          void engineActions.seekDeck(deckId, positionSecs);
        }}
      />
    </div>
  );
});

const DeckPerformanceSection = memo(function DeckPerformanceSection({
  deckId,
  accentKey,
  transportDisabled,
}: {
  deckId: number;
  accentKey: DeckAccent;
  transportDisabled: boolean;
}) {
  const controls = useDeckControls(deckId);
  const transport = useDeckTransport(deckId);
  const cueHeldRef = useRef(false);
  const cueWasHoldRef = useRef(false);
  const isDeckA = accentKey === "a";

  const deck: DeckStatus = {
    ...getDefaultDeck(deckId),
    ...controls,
    ...transport,
  };

  const hotCuePanel = (
    <DeckPadsPanel
      deck={deck}
      disabled={transportDisabled}
      onTriggerHotCue={(slot) => {
        void engineActions.triggerHotCue(deckId, slot);
      }}
      onSaveHotCue={(slot) => {
        void engineActions.saveHotCue(deckId, slot);
      }}
      onDeleteHotCue={(slot) => {
        void engineActions.deleteHotCue(deckId, slot);
      }}
    />
  );

  const loopPanel = (
    <DeckLoopPanel
      deck={deck}
      disabled={transportDisabled}
      onAutoLoop={(beats) => {
        void engineActions.setDeckAutoLoop(deckId, beats);
      }}
      onLoopIn={() => {
        void engineActions.setDeckLoopIn(deckId);
      }}
      onLoopOut={() => {
        void engineActions.setDeckLoopOut(deckId);
      }}
      onExitLoop={() => {
        void engineActions.exitDeckLoop(deckId);
      }}
      onSaveLoop={(slot) => {
        void engineActions.saveLoop(deckId, slot);
      }}
      onRecallSavedLoop={(slot) => {
        void engineActions.recallSavedLoop(deckId, slot);
      }}
      onDeleteLoop={(slot) => {
        void engineActions.deleteLoop(deckId, slot);
      }}
    />
  );

  const togglePlayback = () => {
    if (deck.playing) {
      void engineActions.pauseDeck(deckId);
      return;
    }
    void engineActions.playDeck(deckId);
  };

  const transportControls = (
    <div className="flex shrink-0 flex-col items-center justify-end gap-2">
      <JogPlatter
        accent={accentKey}
        playing={deck.playing}
        bpm={deck.bpm != null ? deck.bpm * deck.speed : deck.bpm}
        hasTrack={Boolean(deck.track)}
        positionSecs={deck.position_secs ?? 0}
        durationSecs={deck.duration_secs}
        speed={deck.speed}
      />
      <div className="flex items-end gap-2">
        {isDeckA ? (
          <>
            <DeckCircularButton
              label="Cue"
              accent={accentKey}
              disabled={transportDisabled}
              title="Hold to audition cue — click without hold to set cue point"
              onPointerDown={() => {
                if (transportDisabled) {
                  return;
                }
                cueWasHoldRef.current = false;
                cueHeldRef.current = true;
                void engineActions.beginDeckCueHold(deckId);
              }}
              onPointerUp={() => {
                if (!cueHeldRef.current) {
                  return;
                }
                cueWasHoldRef.current = true;
                cueHeldRef.current = false;
                void engineActions.endDeckCueHold(deckId);
              }}
              onPointerLeave={() => {
                if (!cueHeldRef.current) {
                  return;
                }
                cueWasHoldRef.current = true;
                cueHeldRef.current = false;
                void engineActions.endDeckCueHold(deckId);
              }}
              onClick={() => {
                if (transportDisabled || cueWasHoldRef.current) {
                  cueWasHoldRef.current = false;
                  return;
                }
                void engineActions.setDeckCuePoint(deckId);
              }}
            />
            <DeckCircularButton
              label={deck.playing ? "Pause" : "Play"}
              accent={accentKey}
              active={deck.playing}
              disabled={transportDisabled}
              onClick={togglePlayback}
            >
              {deck.playing ? (
                <Pause className="size-5" aria-hidden />
              ) : (
                <Play className="size-5 translate-x-0.5" aria-hidden />
              )}
            </DeckCircularButton>
          </>
        ) : (
          <>
            <DeckCircularButton
              label={deck.playing ? "Pause" : "Play"}
              accent={accentKey}
              active={deck.playing}
              disabled={transportDisabled}
              onClick={togglePlayback}
            >
              {deck.playing ? (
                <Pause className="size-5" aria-hidden />
              ) : (
                <Play className="size-5 translate-x-0.5" aria-hidden />
              )}
            </DeckCircularButton>
            <DeckCircularButton
              label="Cue"
              accent={accentKey}
              disabled={transportDisabled}
              title="Hold to audition cue — click without hold to set cue point"
              onPointerDown={() => {
                if (transportDisabled) {
                  return;
                }
                cueWasHoldRef.current = false;
                cueHeldRef.current = true;
                void engineActions.beginDeckCueHold(deckId);
              }}
              onPointerUp={() => {
                if (!cueHeldRef.current) {
                  return;
                }
                cueWasHoldRef.current = true;
                cueHeldRef.current = false;
                void engineActions.endDeckCueHold(deckId);
              }}
              onPointerLeave={() => {
                if (!cueHeldRef.current) {
                  return;
                }
                cueWasHoldRef.current = true;
                cueHeldRef.current = false;
                void engineActions.endDeckCueHold(deckId);
              }}
              onClick={() => {
                if (transportDisabled || cueWasHoldRef.current) {
                  cueWasHoldRef.current = false;
                  return;
                }
                void engineActions.setDeckCuePoint(deckId);
              }}
            />
          </>
        )}
      </div>
    </div>
  );

  const performancePanels = isDeckA ? (
    <>
      {hotCuePanel}
      {loopPanel}
      {transportControls}
    </>
  ) : (
    <>
      {loopPanel}
      {hotCuePanel}
      {transportControls}
    </>
  );

  return (
    <div
      className={cn(
        "flex w-full min-h-36 shrink-0 flex-row items-stretch gap-2 sm:min-h-40",
        isDeckA ? "justify-start" : "justify-end",
      )}
    >
      {performancePanels}
    </div>
  );
});

export function DeckPanel({
  deckId,
  accentKey,
  focused = false,
  onFocus,
}: DeckPanelProps) {
  const accent = DECK_ACCENTS[accentKey];
  const engineRunning = useEngineRunning();
  const busy = useEngineBusy();
  const controls = useDeckControls(deckId);
  const [dragOver, setDragOver] = useState(false);
  const dragDepthRef = useRef(0);

  const loadDisabled = busy || !engineRunning;
  const hasTrack = Boolean(controls.track);
  const dropEnabled = engineRunning && !busy;
  const transportDisabled = busy || !engineRunning || !hasTrack;
  const isDeckA = accentKey === "a";

  const deckForInfo: DeckStatus = {
    ...getDefaultDeck(deckId),
    ...controls,
  };

  useEffect(() => {
    const resetDragState = () => {
      dragDepthRef.current = 0;
      setDragOver(false);
    };

    window.addEventListener("dragend", resetDragState);
    window.addEventListener("drop", resetDragState);
    return () => {
      window.removeEventListener("dragend", resetDragState);
      window.removeEventListener("drop", resetDragState);
    };
  }, []);

  const handleDragEnter = (event: React.DragEvent<HTMLElement>) => {
    if (!dropEnabled || !acceptsTrackDrag(event.dataTransfer)) {
      return;
    }
    event.preventDefault();
    dragDepthRef.current += 1;
    if (dragDepthRef.current === 1) {
      setDragOver(true);
    }
  };

  const handleDragLeave = (event: React.DragEvent<HTMLElement>) => {
    if (!dropEnabled) {
      return;
    }
    event.preventDefault();
    dragDepthRef.current = Math.max(0, dragDepthRef.current - 1);
    if (dragDepthRef.current === 0) {
      setDragOver(false);
    }
  };

  const handleDropTrack = (payload: TrackDragPayload) => {
    if (payload.source === "library" && payload.trackId) {
      void engineActions.loadLibraryTrackToDeck(deckId, payload.trackId);
      return;
    }
    void engineActions.loadPathToDeck(deckId, payload.path);
  };

  const tempoPanel = (
    <DeckTempoPanel
      accent={accentKey}
      deck={deckForInfo}
      disabled={transportDisabled}
      onSpeedChange={(speed) => {
        void engineActions.setDeckSpeed(deckId, speed);
      }}
    />
  );

  return (
    <section
      className={`flex h-full min-h-0 min-w-0 flex-col gap-1 p-2 transition-shadow sm:gap-1.5 sm:p-2.5 ${accent.bg} ${
        dragOver ? "shadow-[inset_0_0_0_2px_rgba(52,211,153,0.55)]" : ""
      } ${focused ? "ring-1 ring-inset ring-white/20" : ""}`}
      onPointerDown={() => onFocus?.()}
      onDragEnter={handleDragEnter}
      onDragLeave={handleDragLeave}
      onDragOver={(event) => {
        if (!dropEnabled || !acceptsTrackDrag(event.dataTransfer)) {
          return;
        }
        event.preventDefault();
        event.dataTransfer.dropEffect = "copy";
      }}
      onDrop={(event) => {
        dragDepthRef.current = 0;
        setDragOver(false);
        if (!dropEnabled) {
          return;
        }
        event.preventDefault();
        const payload = readTrackDragData(event.dataTransfer);
        if (!payload) {
          return;
        }
        handleDropTrack(payload);
      }}
    >
      <div className="flex shrink-0 items-center justify-between gap-2">
        <h2
          className={`text-[10px] font-bold uppercase tracking-widest ${accent.text}`}
        >
          {accent.label}
        </h2>
        <div className="flex shrink-0 items-center gap-1">
          <DeckButton
            type="button"
            active={controls.quantize}
            size="toggle"
            disabled={transportDisabled}
            title={controls.quantize ? "Quantize on" : "Quantize off"}
            onClick={() => {
              void engineActions.setDeckQuantize(deckId, !controls.quantize);
            }}
          >
            Q
          </DeckButton>
          <DeckButton
            type="button"
            size="compact"
            disabled={loadDisabled}
            title={hasTrack ? "Eject track" : "Load track"}
            onClick={() => {
              if (hasTrack) {
                void engineActions.unloadDeck(deckId);
                return;
              }
              void engineActions.pickTrack(deckId);
            }}
          >
            {hasTrack ? "Eject" : "Load"}
          </DeckButton>
        </div>
      </div>

      <DeckTrackInfo deck={deckForInfo} />

      <div className="flex min-h-0 flex-1 gap-2">
        {!isDeckA ? tempoPanel : null}
        <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-2">
          <DeckOverviewSection
            deckId={deckId}
            transportDisabled={transportDisabled}
          />
          <DeckPerformanceSection
            deckId={deckId}
            accentKey={accentKey}
            transportDisabled={transportDisabled}
          />
        </div>
        {isDeckA ? tempoPanel : null}
      </div>
    </section>
  );
}
