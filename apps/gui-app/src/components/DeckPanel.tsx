import { memo, useRef } from "react";
import { Pause, Play } from "lucide-react";
import { cn } from "@/lib/utils";
import { DeckButton } from "@/components/ui/deck-button";
import { type DeckAccent, DECK_ACCENTS } from "@/lib/ui";
import { formatDeckRemainingDisplay, formatDeckTotalDisplay } from "@/lib/format";
import {
  engineActions,
  useDeckControls,
  useDeckOverview,
  useDeckTransport,
  useDeckBusy,
  useEngineRunning,
  useSamplerSlots,
  useSamplerBanks,
  useSamplerEffectivePlayMode,
} from "@/hooks/useEngine";
import { getDefaultDeck } from "@/stores/defaultDeck";
import { useTrack } from "@/hooks/useTrack";
import type { DeckStatus } from "@/types";
import { DeckPadsPanel } from "./DeckPadsPanel";
import { DeckLoopPanel } from "./DeckLoopPanel";
import { DeckOverviewPreview } from "./DeckOverviewPreview";
import { DeckTempoPanel } from "./DeckTempoPanel";
import { DeckTrackInfo } from "./DeckTrackInfo";
import { DeckInfoPopover } from "./DeckInfoPopover";
import { DeckCircularButton, JogPlatter } from "./DeckTransport";
import { TrackDropZone } from "./TrackDropZone";
import { deckDropId } from "@/lib/trackDrag";

interface DeckPanelProps {
  deckId: number;
  accentKey: DeckAccent;
}

const DeckOverviewSection = memo(function DeckOverviewSection({
  deckId,
  transportDisabled,
}: {
  deckId: number;
  transportDisabled: boolean;
}) {
  const overview = useDeckOverview(deckId);
  const { track } = useTrack(overview.track_id);

  return (
    <div className="flex shrink-0 flex-col gap-0.5">
      <div className="flex items-baseline justify-between gap-3 font-mono tabular-nums">
        <span className="text-sm font-semibold text-zinc-100 sm:text-base">
          {formatDeckRemainingDisplay(overview.position_ms, overview.duration_ms)}
        </span>
        <span className="text-[11px] text-zinc-500 sm:text-xs">
          {formatDeckTotalDisplay(overview.duration_ms)}
        </span>
      </div>
      <DeckOverviewPreview
        trackId={overview.track_id}
        path={overview.track}
        positionMs={overview.position_ms ?? 0}
        playing={overview.playing}
        speed={overview.speed}
        durationMs={overview.duration_ms}
        hotCues={track?.hot_cues ?? []}
        disabled={transportDisabled}
        onSeek={(positionMs) => {
          void engineActions.seekDeck(deckId, positionMs);
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
  const { track: libraryTrack } = useTrack(controls.track_id);
  const samplerSlots = useSamplerSlots(deckId);
  const samplerBanks = useSamplerBanks();
  const effectivePlayMode = useSamplerEffectivePlayMode(deckId);
  const cueHeldRef = useRef(false);
  const cueWasHoldRef = useRef(false);
  const isDeckA = accentKey === "a";

  const deck: DeckStatus = {
    ...getDefaultDeck(deckId),
    ...controls,
    ...transport,
    title: libraryTrack?.title ?? controls.title,
    artist: libraryTrack?.artist ?? controls.artist,
    bpm: libraryTrack?.bpm ?? controls.bpm,
    key: libraryTrack?.key ?? controls.key,
    hot_cues: libraryTrack?.hot_cues ?? [],
    saved_loops: libraryTrack?.saved_loops ?? [],
  };

  const hotCuePanel = (
    <DeckPadsPanel
      deck={deck}
      samplerSlots={samplerSlots}
      samplerBanks={samplerBanks}
      effectivePlayMode={effectivePlayMode}
      disabled={transportDisabled}
      onSetPadMode={(mode) => {
        void engineActions.setDeckPadMode(deckId, mode);
      }}
      onTriggerHotCue={(cue) => {
        void engineActions.triggerHotCue(deckId, cue);
      }}
      onSaveHotCue={(slot) => {
        void engineActions.saveHotCue(deckId, slot);
      }}
      onDeleteHotCue={(slot) => {
        if (!deck.track_id) {
          return;
        }
        void engineActions.deleteHotCue(deck.track_id, slot);
      }}
      onBeginLoopRoll={(beats) => {
        void engineActions.beginLoopRoll(deckId, beats);
      }}
      onEndLoopRoll={() => {
        void engineActions.endLoopRoll(deckId);
      }}
      onBeatJump={(beats) => {
        void engineActions.beatJumpDeck(deckId, beats);
      }}
      onTriggerSampler={(slot) => {
        void engineActions.triggerSamplerPad(deckId, slot);
      }}
      onEndSampler={(slot) => {
        void engineActions.endSamplerPad(deckId, slot);
      }}
      onClearSamplerSlot={(slot) => {
        void engineActions.clearSamplerSlot(slot, deckId);
      }}
      onSelectSamplerBank={(bankId) => {
        void engineActions.setDeckSamplerBank(deckId, bankId);
      }}
      onSaveSamplerBank={(bankId, name, playMode) => {
        void engineActions.updateSamplerBank(bankId, name, playMode);
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
      onTriggerLoop={(loop) => {
        void engineActions.triggerLoop(deckId, loop);
      }}
      onDeleteLoop={(slot) => {
        if (!deck.track_id) {
          return;
        }
        void engineActions.deleteLoop(deck.track_id, slot);
      }}
      onBeatJump={(beats) => {
        void engineActions.beatJumpDeck(deckId, beats);
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
        enabled={!transportDisabled}
        jogTouching={deck.jog_touching}
        positionMs={deck.position_ms ?? 0}
        durationMs={deck.duration_ms}
        speed={deck.speed}
        onJogTouch={(touching) => {
          void engineActions.jogTouch(deckId, touching);
        }}
        onJogTurn={(delta) => {
          void engineActions.jogTurn(deckId, delta);
        }}
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

  return (
    <div
      className={cn(
        "flex w-full min-h-36 shrink-0 items-stretch gap-2 sm:min-h-40",
        isDeckA ? "flex-row justify-start" : "flex-row-reverse justify-start",
      )}
    >
      {hotCuePanel}
      {loopPanel}
      {transportControls}
    </div>
  );
});

export function DeckPanel({ deckId, accentKey }: DeckPanelProps) {
  const accent = DECK_ACCENTS[accentKey];
  const engineRunning = useEngineRunning();
  const deckBusy = useDeckBusy(deckId);
  const controls = useDeckControls(deckId);

  const loadDisabled = deckBusy || !engineRunning;
  const hasTrack = Boolean(controls.track);
  const dropEnabled = engineRunning && !deckBusy;
  const transportDisabled = deckBusy || !engineRunning || !hasTrack;
  const isDeckA = accentKey === "a";

  const deckForInfo: DeckStatus = {
    ...getDefaultDeck(deckId),
    ...controls,
  };

  const tempoPanel = (
    <DeckTempoPanel
      accent={accentKey}
      deck={deckForInfo}
      disabled={transportDisabled}
      onSpeedChange={(speed) => {
        void engineActions.setDeckSpeed(deckId, speed);
      }}
      onToggleSync={(beatSync) => {
        void engineActions.toggleDeckSync(deckId, beatSync);
      }}
      onSetMaster={() => {
        void engineActions.setMasterDeck(deckId);
      }}
    />
  );

  return (
    <TrackDropZone
      id={deckDropId(deckId)}
      data={{ type: "deck", deckId }}
      disabled={!dropEnabled}
      collisionPriority={0}
      className={cn(
        "flex h-full min-h-0 min-w-0 flex-col gap-1 p-2 transition-shadow sm:gap-1.5 sm:p-2.5",
        accent.bg,
      )}
    >
      <div className="flex shrink-0 items-center justify-between gap-2">
        <div className="flex min-w-0 items-center gap-1">
          <h2 className={`text-[10px] font-bold uppercase tracking-widest ${accent.text}`}>
            {accent.label}
          </h2>
          <DeckInfoPopover deck={deckForInfo} disabled={!hasTrack} accentClass={accent.text} />
        </div>
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
          <DeckOverviewSection deckId={deckId} transportDisabled={transportDisabled} />
          <DeckPerformanceSection
            deckId={deckId}
            accentKey={accentKey}
            transportDisabled={transportDisabled}
          />
        </div>
        {isDeckA ? tempoPanel : null}
      </div>
    </TrackDropZone>
  );
}
