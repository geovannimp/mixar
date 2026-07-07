import { useCallback, useRef, type KeyboardEvent, type PointerEvent } from "react";
import { cn } from "@/lib/utils";
import { EQ_MAX_DB, EQ_MIN_DB, snapEqDb } from "@/lib/eq";

interface RotaryKnobProps {
  label: string;
  value: number;
  min?: number;
  max?: number;
  disabled?: boolean;
  accentClass?: string;
  ringClass?: string;
  className?: string;
  onValueChange: (value: number) => void;
}

function valueToAngle(value: number, min: number, max: number): number {
  const t = (value - min) / (max - min);
  return t * 270 - 135;
}

export function RotaryKnob({
  label,
  value,
  min = EQ_MIN_DB,
  max = EQ_MAX_DB,
  disabled,
  accentClass,
  ringClass,
  className,
  onValueChange,
}: RotaryKnobProps) {
  const dragRef = useRef<{ startY: number; startValue: number } | null>(null);
  const angle = valueToAngle(value, min, max);

  const updateFromPointer = useCallback(
    (clientY: number) => {
      const drag = dragRef.current;
      if (!drag) {
        return;
      }
      const deltaY = drag.startY - clientY;
      const range = max - min;
      const next = snapEqDb(drag.startValue + (deltaY / 72) * range);
      onValueChange(next);
    },
    [max, min, onValueChange],
  );

  const handlePointerDown = (event: PointerEvent<HTMLButtonElement>) => {
    if (disabled) {
      return;
    }
    event.currentTarget.setPointerCapture(event.pointerId);
    dragRef.current = { startY: event.clientY, startValue: value };
  };

  const handlePointerMove = (event: PointerEvent<HTMLButtonElement>) => {
    if (!dragRef.current) {
      return;
    }
    updateFromPointer(event.clientY);
  };

  const handlePointerUp = (event: PointerEvent<HTMLButtonElement>) => {
    if (!dragRef.current) {
      return;
    }
    dragRef.current = null;
    event.currentTarget.releasePointerCapture(event.pointerId);
  };

  const handleKeyDown = (event: KeyboardEvent<HTMLButtonElement>) => {
    if (disabled) {
      return;
    }
    if (event.key === "ArrowUp" || event.key === "ArrowRight") {
      event.preventDefault();
      onValueChange(snapEqDb(value + 1));
    }
    if (event.key === "ArrowDown" || event.key === "ArrowLeft") {
      event.preventDefault();
      onValueChange(snapEqDb(value - 1));
    }
    if (event.key === "Home") {
      event.preventDefault();
      onValueChange(min);
    }
    if (event.key === "End") {
      event.preventDefault();
      onValueChange(max);
    }
  };

  const displayValue = value > 0 ? `+${value}` : `${value}`;

  return (
    <div className={cn("flex flex-col items-center gap-0.5", className)}>
      <span
        className={cn(
          "text-[8px] font-semibold uppercase tracking-wide",
          accentClass ?? "text-zinc-500",
        )}
      >
        {label}
      </span>
      <button
        type="button"
        disabled={disabled}
        aria-label={`${label} EQ`}
        aria-valuemin={min}
        aria-valuemax={max}
        aria-valuenow={value}
        role="slider"
        className={cn(
          "relative size-8 touch-none rounded-full border-2 bg-zinc-900/90 shadow-inner outline-none select-none",
          "transition-[box-shadow,scale] hover:bg-zinc-800/90",
          "focus-visible:ring-2 focus-visible:ring-ring/40",
          "disabled:cursor-not-allowed disabled:opacity-45",
          "active:scale-105",
          ringClass ?? "border-zinc-600",
        )}
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
        onPointerCancel={handlePointerUp}
        onKeyDown={handleKeyDown}
      >
        <span
          aria-hidden
          className="absolute inset-1 rounded-full border border-white/6"
        />
        <span
          aria-hidden
          className="absolute top-1/2 left-1/2 h-[38%] w-0.5 origin-bottom rounded-full bg-zinc-100 shadow-[0_0_5px_rgba(255,255,255,0.3)]"
          style={{ transform: `translateX(-50%) translateY(-100%) rotate(${angle}deg)` }}
        />
        <span
          aria-hidden
          className="absolute top-1/2 left-1/2 size-1.5 -translate-x-1/2 -translate-y-1/2 rounded-full bg-zinc-500"
        />
      </button>
      <span className="min-w-[3ch] text-center text-[8px] tabular-nums text-zinc-500">
        {displayValue}
      </span>
    </div>
  );
}
