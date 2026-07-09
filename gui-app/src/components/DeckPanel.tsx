import { useEffect, useRef, useState } from "react";
import { Pause, Play } from "lucide-react";
import {
  acceptsTrackDrag,
  readTrackDragData,
  type TrackDragPayload,
} from "../lib/libraryTable";
import { buttonCompact, type DeckAccent, DECK_ACCENTS } from "../lib/ui";
import type { DeckStatus } from "../types";
import { DeckOverviewPreview } from "./DeckOverviewPreview";
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
  onDropTrack,
}: DeckPanelProps) {
  const [dragOver, setDragOver] = useState(false);
  const dragDepthRef = useRef(0);
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
          onClick={onPickTrack}
        >
          Load
        </button>
      </div>

      <DeckTrackInfo deck={deck} />

      <div className="flex min-h-0 flex-1 gap-2">
        <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-2">
          <DeckOverviewPreview
            trackId={deck.track_id}
            path={deck.track}
            positionSecs={deck.position_secs ?? 0}
            durationSecs={deck.duration_secs}
            eq={deck.eq}
          />

          <div className="mt-auto flex justify-center">
            <div className="flex flex-col items-center gap-2">
              <JogPlatter
                accent={accentKey}
                playing={deck.playing}
                bpm={deck.bpm}
                hasTrack={hasTrack}
              />
              <div className="flex items-end gap-2">
                <DeckCircularButton
                  label="Cue"
                  accent={accentKey}
                  disabled={true}
                  title="Coming in Phase 2"
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
              </div>
            </div>
          </div>
        </div>

        <DeckTempoPanel
          accent={accentKey}
          deck={deck}
          disabled={transportDisabled}
          onSpeedChange={onSpeedChange}
        />
      </div>
    </section>
  );
}
