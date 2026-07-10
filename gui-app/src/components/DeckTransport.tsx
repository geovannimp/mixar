import type { ReactNode } from "react";
import { useEffect, useRef } from "react";
import { animate, motion, useMotionValue, useTransform } from "motion/react";
import { DeckButton } from "@/components/ui/deck-button";
import { barCycleRotationDeg, getBarCycleDurationSecs } from "../lib/format";
import { useSmoothTrackProgress } from "../hooks/useSmoothTrackProgress";
import { type DeckAccent, DECK_ACCENTS } from "../lib/ui";

interface JogPlatterProps {
  accent: DeckAccent;
  playing: boolean;
  bpm: number | null;
  hasTrack: boolean;
  positionSecs?: number;
  durationSecs?: number | null;
  speed?: number;
}

/** Flat jog wheel — bar tracker follows tempo; outer ring shows track progress. */
export function JogPlatter({
  accent: accentKey,
  playing,
  bpm,
  hasTrack,
  positionSecs = 0,
  durationSecs,
  speed = 1,
}: JogPlatterProps) {
  const accent = DECK_ACCENTS[accentKey];
  const trackerRotate = useMotionValue(0);
  const lastPositionRef = useRef(0);
  const rotationRef = useRef(0);
  const trackerInitializedRef = useRef(false);

  const trackProgress = useSmoothTrackProgress({
    positionSecs,
    durationSecs,
    playing,
    speed,
  });

  const ringRadius = 46;
  const ringCircumference = 2 * Math.PI * ringRadius;
  const ringDashoffset = useTransform(
    trackProgress,
    (progress) => ringCircumference * (1 - progress),
  );
  const ringStroke =
    accentKey === "a" ? "rgba(56, 189, 248, 0.55)" : "rgba(251, 113, 133, 0.55)";

  const effectiveBpm = bpm != null && bpm > 0 ? bpm : 120;

  useEffect(() => {
    if (!hasTrack) {
      trackerInitializedRef.current = false;
      lastPositionRef.current = 0;
      rotationRef.current = 0;
      trackerRotate.set(0);
      return;
    }

    const cycleDuration = getBarCycleDurationSecs(effectiveBpm);
    if (cycleDuration == null) {
      return;
    }

    if (!trackerInitializedRef.current) {
      trackerInitializedRef.current = true;
      lastPositionRef.current = positionSecs;
      rotationRef.current = barCycleRotationDeg(positionSecs, effectiveBpm);
      trackerRotate.set(rotationRef.current);
      return;
    }

    const delta = positionSecs - lastPositionRef.current;
    lastPositionRef.current = positionSecs;
    const seekThreshold = Math.max(0.2, cycleDuration * 0.15);
    const isSeek = Math.abs(delta) > seekThreshold;

    if (isSeek) {
      rotationRef.current = barCycleRotationDeg(positionSecs, effectiveBpm);
    } else {
      rotationRef.current += (delta / cycleDuration) * 360;
    }

    void animate(trackerRotate, rotationRef.current, {
      duration: isSeek ? 0 : 0.15,
      ease: "linear",
    });
  }, [positionSecs, effectiveBpm, hasTrack, trackerRotate]);

  return (
    <div
      className="relative size-32 shrink-0 sm:size-36"
      title="Jog wheel"
      aria-label="Jog wheel"
    >
      <div
        className={`relative flex size-full items-center justify-center overflow-hidden rounded-full border bg-zinc-950/80 shadow-inner ${accent.ring}`}
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
            hasTrack
              ? accentKey === "a"
                ? "bg-sky-400"
                : "bg-rose-400"
              : "bg-zinc-500"
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
