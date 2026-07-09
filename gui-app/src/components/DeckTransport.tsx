import type { CSSProperties, ReactNode } from "react";
import { useEffect, useState } from "react";
import { type DeckAccent, DECK_ACCENTS } from "../lib/ui";

interface JogPlatterProps {
  accent: DeckAccent;
  playing: boolean;
  bpm: number | null;
  hasTrack: boolean;
}

/** Vinyl-style jog wheel — rotation follows track tempo; center shows a spindle dot, not BPM. */
export function JogPlatter({ accent: accentKey, playing, bpm, hasTrack }: JogPlatterProps) {
  const accent = DECK_ACCENTS[accentKey];
  const [spinKey, setSpinKey] = useState(0);

  useEffect(() => {
    if (playing) {
      setSpinKey((value) => value + 1);
    }
  }, [playing]);

  const effectiveBpm = bpm != null && bpm > 0 ? bpm : 120;
  const spinDurationSec = 60 / effectiveBpm;

  return (
    <div
      className={`relative flex size-14 shrink-0 items-center justify-center rounded-full border-2 bg-zinc-950/90 shadow-inner sm:size-16 ${accent.ring}`}
      title="Jog wheel"
      aria-label="Jog wheel"
    >
      <div
        key={spinKey}
        className={`absolute inset-[5px] rounded-full border border-white/10 bg-gradient-to-br ${accent.waveform} ${playing ? "animate-deck-platter" : ""}`}
        style={
          playing
            ? ({ animationDuration: `${spinDurationSec}s` } satisfies CSSProperties)
            : undefined
        }
      />
      <div className="relative z-10 size-2.5 rounded-full bg-zinc-300 shadow-[0_0_4px_rgba(255,255,255,0.35)]" />
      {!hasTrack ? (
        <span className="absolute -bottom-3 text-[7px] font-semibold uppercase tracking-wider text-zinc-600">
          Jog
        </span>
      ) : null}
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
  children?: ReactNode;
  variant?: "default" | "play";
}

export function DeckCircularButton({
  label,
  accent,
  disabled,
  active,
  title,
  onClick,
  children,
  variant = "default",
}: DeckCircularButtonProps) {
  const accentStyles = DECK_ACCENTS[accent];
  const isPlay = variant === "play";

  return (
    <button
      type="button"
      className="flex shrink-0 flex-col items-center gap-0.5 disabled:cursor-not-allowed disabled:opacity-45"
      disabled={disabled}
      aria-label={label}
      title={title ?? label}
      onClick={onClick}
    >
      <span
        className={`inline-flex size-11 items-center justify-center rounded-full border-2 text-sm font-bold shadow-md transition sm:size-12 ${
          isPlay && active
            ? "border-emerald-400/70 bg-emerald-500/25 text-emerald-100"
            : isPlay
              ? `${accentStyles.button} ${accentStyles.ring}`
              : active
                ? `${accentStyles.button} ${accentStyles.ring} ${accentStyles.text}`
                : "border-white/15 bg-zinc-900/90 text-zinc-300 hover:bg-zinc-800/90"
        }`}
      >
        {children ?? (
          <span className="text-[9px] font-bold uppercase tracking-wide sm:text-[10px]">
            {label}
          </span>
        )}
      </span>
      <span className="text-[8px] font-semibold uppercase tracking-widest text-zinc-500">
        {label}
      </span>
    </button>
  );
}
