import { useEffect, useRef, useState } from "react";
import { Pause, Play } from "lucide-react";
import { cn } from "@/lib/utils";
import {
  acceptsTrackDrag,
  readTrackDragData,
  type TrackDragPayload,
} from "../lib/libraryTable";
import { buttonCompact, type DeckAccent, DECK_ACCENTS } from "../lib/ui";
import type { DeckStatus } from "../types";
import { DeckPadsPanel } from "./DeckPadsPanel";
import { DeckLoopPanel } from "./DeckLoopPanel";
import { DeckOverviewPreview } from "./DeckOverviewPreview";
import { DeckPerformancePanel } from "./DeckPerformancePanel";
import { DeckTempoPanel } from "./DeckTempoPanel";
import { DeckTrackInfo } from "./DeckTrackInfo";
import { DeckCircularButton, JogPlatter } from "./DeckTransport";

interface DeckPanelProps {
  accent: (typeof DECK_ACCENTS)[DeckAccent];
  accentKey: DeckAccent;
  deck: DeckStatus;
  engineRunning: boolean;
  busy: boolean;
  onPickTrack: () => void;
  onTogglePlayback: () => void;
  onSpeedChange: (speed: number) => void;
  onSeek: (positionSecs: number) => void;
  onSetCuePoint: () => void;
  onBeginCueHold: () => void;
  onEndCueHold: () => void;
  onTriggerHotCue: (slot: number) => void;
  onSaveHotCue: (slot: number) => void;
  onDeleteHotCue: (slot: number) => void;
  onAutoLoop: (beats: number) => void;
  onLoopIn: () => void;
  onLoopOut: () => void;
  onExitLoop: () => void;
  onToggleQuantize: (enabled: boolean) => void;
  onUnload: () => void;
  onDropTrack?: (payload: TrackDragPayload) => void;
}

export function DeckPanel({
  accent,
  accentKey,
  deck,
  engineRunning,
  busy,
  onPickTrack,
  onTogglePlayback,
  onSpeedChange,
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
  onDropTrack,
}: DeckPanelProps) {
  const [dragOver, setDragOver] = useState(false);
  const dragDepthRef = useRef(0);
  const cueHeldRef = useRef(false);
  const cueWasHoldRef = useRef(false);
  const loadDisabled = busy || !engineRunning;
  const hasTrack = Boolean(deck.track);
  const dropEnabled = Boolean(onDropTrack) && engineRunning && !busy;
  const transportDisabled = busy || !engineRunning || !hasTrack;

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

  const isDeckA = accentKey === "a";

  const tempoPanel = (
    <DeckTempoPanel
      accent={accentKey}
      deck={deck}
      disabled={transportDisabled}
      onSpeedChange={onSpeedChange}
    />
  );

  const hotCuePanel = (
    <DeckPadsPanel
      deck={deck}
      disabled={transportDisabled}
      onTriggerHotCue={onTriggerHotCue}
      onSaveHotCue={onSaveHotCue}
      onDeleteHotCue={onDeleteHotCue}
    />
  );

  const loopPanel = (
    <DeckLoopPanel
      deck={deck}
      disabled={transportDisabled}
      onAutoLoop={onAutoLoop}
      onLoopIn={onLoopIn}
      onLoopOut={onLoopOut}
      onExitLoop={onExitLoop}
    />
  );

  const transportControls = (
    <div className="flex shrink-0 flex-col items-center justify-end gap-2">
      <JogPlatter
        accent={accentKey}
        playing={deck.playing}
        bpm={deck.bpm != null ? deck.bpm * deck.speed : deck.bpm}
        hasTrack={hasTrack}
        positionSecs={deck.position_secs ?? 0}
        durationSecs={deck.duration_secs}
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
                void onBeginCueHold();
              }}
              onPointerUp={() => {
                if (!cueHeldRef.current) {
                  return;
                }
                cueWasHoldRef.current = true;
                cueHeldRef.current = false;
                void onEndCueHold();
              }}
              onPointerLeave={() => {
                if (!cueHeldRef.current) {
                  return;
                }
                cueWasHoldRef.current = true;
                cueHeldRef.current = false;
                void onEndCueHold();
              }}
              onClick={() => {
                if (transportDisabled || cueWasHoldRef.current) {
                  cueWasHoldRef.current = false;
                  return;
                }
                void onSetCuePoint();
              }}
            />
            <DeckCircularButton
              label={deck.playing ? "Pause" : "Play"}
              accent={accentKey}
              variant="play"
              active={deck.playing}
              disabled={transportDisabled}
              onClick={onTogglePlayback}
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
              variant="play"
              active={deck.playing}
              disabled={transportDisabled}
              onClick={onTogglePlayback}
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
                void onBeginCueHold();
              }}
              onPointerUp={() => {
                if (!cueHeldRef.current) {
                  return;
                }
                cueWasHoldRef.current = true;
                cueHeldRef.current = false;
                void onEndCueHold();
              }}
              onPointerLeave={() => {
                if (!cueHeldRef.current) {
                  return;
                }
                cueWasHoldRef.current = true;
                cueHeldRef.current = false;
                void onEndCueHold();
              }}
              onClick={() => {
                if (transportDisabled || cueWasHoldRef.current) {
                  cueWasHoldRef.current = false;
                  return;
                }
                void onSetCuePoint();
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

  const mainColumn = (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-2">
      <DeckOverviewPreview
        trackId={deck.track_id}
        path={deck.track}
        positionSecs={deck.position_secs ?? 0}
        durationSecs={deck.duration_secs}
        hotCues={deck.hot_cues}
        activeLoop={deck.active_loop}
        disabled={transportDisabled}
        onSeek={onSeek}
      />

      <div
        className={cn(
          "flex w-full min-h-36 shrink-0 flex-row items-stretch gap-2 sm:min-h-40",
          isDeckA ? "justify-start" : "justify-end",
        )}
      >
        {performancePanels}
      </div>
    </div>
  );

  return (
    <section
      className={`flex h-full min-h-0 min-w-0 flex-col gap-1 p-2 transition-shadow sm:gap-1.5 sm:p-2.5 ${accent.bg} ${
        dragOver ? "shadow-[inset_0_0_0_2px_rgba(52,211,153,0.55)]" : ""
      }`}
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
        onDropTrack?.(payload);
      }}
    >
      <div className="flex shrink-0 items-center justify-between gap-2">
        <h2
          className={`text-[10px] font-bold uppercase tracking-widest ${accent.text}`}
        >
          {accent.label}
        </h2>
        <button
          type="button"
          className={`${buttonCompact} shrink-0 border-white/10 bg-black/30 text-zinc-400 hover:bg-black/45`}
          disabled={loadDisabled}
          title={hasTrack ? "Eject track" : "Load track"}
          onClick={hasTrack ? onUnload : onPickTrack}
        >
          {hasTrack ? "Eject" : "Load"}
        </button>
      </div>

      <DeckTrackInfo deck={deck} />

      <DeckPerformancePanel
        deck={deck}
        disabled={transportDisabled}
        onToggleQuantize={onToggleQuantize}
      />

      <div className="flex min-h-0 flex-1 gap-2">
        {!isDeckA ? tempoPanel : null}
        {mainColumn}
        {isDeckA ? tempoPanel : null}
      </div>
    </section>
  );
}
