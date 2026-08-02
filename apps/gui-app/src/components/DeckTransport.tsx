import type { ReactNode, PointerEvent as ReactPointerEvent } from "react";
import { useEffect, useRef } from "react";
import { motion, useMotionValue, useTransform } from "motion/react";
import { DeckButton } from "@/components/ui/deck-button";
import { barCycleRotationDeg, getBarCycleDurationMs } from "@/lib/format";
import { degreesToJogTicks } from "@/lib/jogTicks";
import { useSmoothTrackProgress } from "@/hooks/useSmoothTrackProgress";
import { type DeckAccent, DECK_ACCENTS } from "@/lib/ui";

interface JogPlatterProps {
  accent: DeckAccent;
  playing: boolean;
  bpm: number | null;
  hasTrack: boolean;
  enabled?: boolean;
  jogTouching?: boolean;
  positionMs?: number;
  durationMs?: number | null;
  speed?: number;
  onJogTouch?: (touching: boolean) => void;
  onJogTurn?: (delta: number) => void;
}

function pointerAngleDeg(el: HTMLElement, clientX: number, clientY: number): number {
  const rect = el.getBoundingClientRect();
  const cx = rect.left + rect.width / 2;
  const cy = rect.top + rect.height / 2;
  return (Math.atan2(clientY - cy, clientX - cx) * 180) / Math.PI;
}

/** Flat jog wheel — always top plate; drag publishes jog_touch + jog_turn. */
export function JogPlatter({
  accent: accentKey,
  playing,
  bpm,
  hasTrack,
  enabled = false,
  jogTouching = false,
  positionMs = 0,
  durationMs,
  speed = 1,
  onJogTouch,
  onJogTurn,
}: JogPlatterProps) {
  const accent = DECK_ACCENTS[accentKey];
  const trackerRotate = useMotionValue(0);
  const lastPositionRef = useRef(0);
  const rotationRef = useRef(0);
  const trackerInitializedRef = useRef(false);
  const draggingRef = useRef(false);
  const lastAngleRef = useRef<number | null>(null);

  const trackProgress = useSmoothTrackProgress({
    positionMs,
    durationMs,
    playing,
    speed,
  });

  const ringRadius = 46;
  const ringCircumference = 2 * Math.PI * ringRadius;
  const ringDashoffset = useTransform(
    trackProgress,
    (progress) => ringCircumference * (1 - progress),
  );
  const ringStroke = accentKey === "a" ? "rgba(56, 189, 248, 0.55)" : "rgba(251, 113, 133, 0.55)";

  const effectiveBpm = bpm != null && bpm > 0 ? bpm : 120;
  const interactive = enabled && hasTrack;

  useEffect(() => {
    if (!hasTrack) {
      trackerInitializedRef.current = false;
      lastPositionRef.current = 0;
      rotationRef.current = 0;
      trackerRotate.set(0);
      return;
    }

    const cycleDurationMs = getBarCycleDurationMs(effectiveBpm);
    if (cycleDurationMs == null) {
      return;
    }

    if (!trackerInitializedRef.current) {
      trackerInitializedRef.current = true;
      lastPositionRef.current = positionMs;
      rotationRef.current = barCycleRotationDeg(positionMs, effectiveBpm);
      trackerRotate.set(rotationRef.current);
      return;
    }

    if (draggingRef.current || jogTouching) {
      lastPositionRef.current = positionMs;
      return;
    }

    const delta = positionMs - lastPositionRef.current;
    lastPositionRef.current = positionMs;
    const seekThreshold = Math.max(200, cycleDurationMs * 0.15);
    const isSeek = Math.abs(delta) > seekThreshold;

    if (isSeek) {
      rotationRef.current = barCycleRotationDeg(positionMs, effectiveBpm);
    } else {
      rotationRef.current += (delta / cycleDurationMs) * 360;
    }

    // Direct set — avoid overlapping animate() calls at ~30 Hz position ticks.
    trackerRotate.set(rotationRef.current);
  }, [positionMs, effectiveBpm, hasTrack, trackerRotate, jogTouching]);

  const onPointerDown = (event: ReactPointerEvent<HTMLDivElement>) => {
    if (!interactive || event.button !== 0) {
      return;
    }
    const el = event.currentTarget;
    el.setPointerCapture(event.pointerId);
    draggingRef.current = true;
    lastAngleRef.current = pointerAngleDeg(el, event.clientX, event.clientY);
    onJogTouch?.(true);
  };

  const onPointerMove = (event: ReactPointerEvent<HTMLDivElement>) => {
    if (!draggingRef.current || !interactive) {
      return;
    }
    const el = event.currentTarget;
    const angle = pointerAngleDeg(el, event.clientX, event.clientY);
    const prev = lastAngleRef.current;
    lastAngleRef.current = angle;
    if (prev == null) {
      return;
    }
    let deltaDeg = angle - prev;
    if (deltaDeg > 180) {
      deltaDeg -= 360;
    } else if (deltaDeg < -180) {
      deltaDeg += 360;
    }
    rotationRef.current += deltaDeg;
    trackerRotate.set(rotationRef.current);
    const ticks = degreesToJogTicks(deltaDeg);
    if (ticks !== 0) {
      onJogTurn?.(ticks);
    }
  };

  const onPointerUp = (event: ReactPointerEvent<HTMLDivElement>) => {
    if (!draggingRef.current) {
      return;
    }
    draggingRef.current = false;
    lastAngleRef.current = null;
    try {
      event.currentTarget.releasePointerCapture(event.pointerId);
    } catch {
      // ignore
    }
    onJogTouch?.(false);
  };

  return (
    <div
      className={`relative size-32 shrink-0 sm:size-36 ${interactive ? "cursor-grab touch-none active:cursor-grabbing" : ""}`}
      title="Jog wheel"
      aria-label="Jog wheel"
      role={interactive ? "slider" : undefined}
      aria-disabled={!interactive}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onPointerCancel={onPointerUp}
      onLostPointerCapture={() => {
        if (draggingRef.current) {
          draggingRef.current = false;
          lastAngleRef.current = null;
          onJogTouch?.(false);
        }
      }}
    >
      <div
        className={`relative flex size-full items-center justify-center overflow-hidden rounded-full border bg-zinc-950/80 shadow-inner ${accent.ring} ${
          jogTouching ? "ring-2 ring-white/30" : ""
        }`}
      >
        <svg
          className="pointer-events-none absolute inset-0 size-full -rotate-90"
          viewBox="0 0 100 100"
          aria-hidden
        >
          <circle
            cx="50"
            cy="50"
            r={ringRadius}
            fill="none"
            stroke="rgba(255,255,255,0.06)"
            strokeWidth="2"
          />
          {hasTrack ? (
            <motion.circle
              cx="50"
              cy="50"
              r={ringRadius}
              fill="none"
              stroke={ringStroke}
              strokeWidth="2"
              strokeLinecap="round"
              strokeDasharray={ringCircumference}
              style={{ strokeDashoffset: ringDashoffset }}
            />
          ) : null}
        </svg>

        <div className="absolute inset-3 rounded-full border border-white/10 bg-zinc-900/90 sm:inset-3.5">
          <div
            className={`absolute inset-0 rounded-full bg-linear-to-br opacity-25 ${accent.waveform}`}
          />
        </div>

        {hasTrack ? (
          <motion.div
            className={`pointer-events-none absolute left-1/2 top-1/2 z-[1] h-[38%] w-0.5 origin-top rounded-full ${
              accentKey === "a" ? "bg-sky-400" : "bg-rose-400"
            }`}
            style={{ x: "-50%", rotate: trackerRotate }}
            aria-hidden
          />
        ) : null}

        <div
          className={`relative z-10 size-2 rounded-full sm:size-2.5 ${
            hasTrack ? (accentKey === "a" ? "bg-sky-400" : "bg-rose-400") : "bg-zinc-500"
          }`}
        />
      </div>
    </div>
  );
}

interface DeckCircularButtonProps {
  label: string;
  accent: DeckAccent;
  disabled?: boolean;
  active?: boolean;
  title?: string;
  onClick?: () => void;
  onPointerDown?: () => void;
  onPointerUp?: () => void;
  onPointerLeave?: () => void;
  children?: ReactNode;
}

export function DeckCircularButton({
  label,
  accent,
  disabled,
  active,
  title,
  onClick,
  onPointerDown,
  onPointerUp,
  onPointerLeave,
  children,
}: DeckCircularButtonProps) {
  return (
    <div className="flex shrink-0 flex-col items-center gap-0.5">
      <DeckButton
        type="button"
        active={active}
        accent={accent}
        size="circular"
        disabled={disabled}
        aria-label={label}
        title={title ?? label}
        onClick={onClick}
        onPointerDown={onPointerDown}
        onPointerUp={onPointerUp}
        onPointerLeave={onPointerLeave}
      >
        {children ?? (
          <span className="text-[9px] font-bold uppercase tracking-wide sm:text-[10px]">
            {label}
          </span>
        )}
      </DeckButton>
      <span className="text-[8px] font-semibold uppercase tracking-widest text-zinc-500">
        {label}
      </span>
    </div>
  );
}
