import {
  useCallback,
  useRef,
  type KeyboardEvent,
  type PointerEvent,
} from "react";
import { cn } from "@/lib/utils";
import { EQ_MAX_DB, EQ_MIN_DB, EQ_STEP_DB } from "@/lib/eq";

interface RotaryKnobProps {
  label: string;
  value: number;
  min?: number;
  max?: number;
  step?: number;
  /** When set, value fill grows from this detent (e.g. 0.5 for ±dB norms). */
  center?: number;
  disabled?: boolean;
  ariaLabel?: string;
  accentClass?: string;
  ringClass?: string;
  className?: string;
  /** Visual size of the dial; default fits mixer strips, `sm` fits the title bar. */
  size?: "md" | "sm";
  onValueChange: (value: number) => void;
}

/** Travel arc matches typical DJ pots: -135° … +135° (270° total). */
const ANGLE_MIN_DEG = -135;
const ANGLE_SPAN_DEG = 270;
const ANGLE_MAX_DEG = ANGLE_MIN_DEG + ANGLE_SPAN_DEG;

/** CSS angle: 0° = up, clockwise → SVG point in a square viewBox. */
function polarToSvg(
  angleDeg: number,
  radius: number,
  center = 50,
): { x: number; y: number } {
  const rad = (angleDeg * Math.PI) / 180;
  return {
    x: center + radius * Math.sin(rad),
    y: center - radius * Math.cos(rad),
  };
}

/** Clockwise arc path from `fromDeg` to `toDeg` (CSS angles). */
function clockwiseArcPath(
  fromDeg: number,
  toDeg: number,
  radius: number,
): string | null {
  const span = toDeg - fromDeg;
  if (span <= 0.05) {
    return null;
  }
  const start = polarToSvg(fromDeg, radius);
  const end = polarToSvg(toDeg, radius);
  const largeArc = span > 180 ? 1 : 0;
  return `M ${start.x} ${start.y} A ${radius} ${radius} 0 ${largeArc} 1 ${end.x} ${end.y}`;
}

function TravelArcs({
  fillFromDeg,
  fillToDeg,
  fillClassName,
  strokeWidth,
}: {
  fillFromDeg: number;
  fillToDeg: number;
  fillClassName?: string;
  strokeWidth: number;
}) {
  const radius = 50 - strokeWidth / 2;
  const trackPath = clockwiseArcPath(ANGLE_MIN_DEG, ANGLE_MAX_DEG, radius);
  const valuePath = clockwiseArcPath(fillFromDeg, fillToDeg, radius);
  const fillStroke =
    fillClassName?.replaceAll("border-", "stroke-") ?? "stroke-zinc-300";

  return (
    <svg aria-hidden viewBox="0 0 100 100" className=" inset-0 size-full">
      {trackPath ? (
        <path
          d={trackPath}
          fill="none"
          strokeWidth={strokeWidth}
          strokeLinecap="butt"
          className="fill-none stroke-zinc-700/30"
        />
      ) : null}
      {valuePath ? (
        <path
          d={valuePath}
          fill="none"
          strokeWidth={strokeWidth}
          strokeLinecap="butt"
          className={cn("fill-none", fillStroke)}
        />
      ) : null}
    </svg>
  );
}

/** Fill from `center` when provided (or true bipolar min/max); otherwise from min. */
function valueFillAngles(
  value: number,
  min: number,
  max: number,
  center?: number,
): { from: number; to: number } {
  const valueAngle = valueToAngle(value, min, max);
  const detent = center ?? (min < 0 && max > 0 ? 0 : undefined);
  if (detent === undefined) {
    return { from: ANGLE_MIN_DEG, to: valueAngle };
  }
  const zeroAngle = valueToAngle(detent, min, max);
  if (valueAngle >= zeroAngle) {
    return { from: zeroAngle, to: valueAngle };
  }
  return { from: valueAngle, to: zeroAngle };
}

function valueToAngle(value: number, min: number, max: number): number {
  const t = (value - min) / (max - min);
  return t * ANGLE_SPAN_DEG + ANGLE_MIN_DEG;
}

function snapToStep(value: number, step: number): number {
  if (step <= 0) {
    return value;
  }
  const snapped = Math.round(value / step) * step;
  return Object.is(snapped, -0) ? 0 : snapped;
}

export function RotaryKnob({
  label,
  value,
  min = EQ_MIN_DB,
  max = EQ_MAX_DB,
  step = EQ_STEP_DB,
  center,
  disabled,
  ariaLabel,
  accentClass,
  ringClass,
  className,
  size = "md",
  onValueChange,
}: RotaryKnobProps) {
  const dragRef = useRef<{ startY: number; startValue: number } | null>(null);
  const snappedValue = snapToStep(value, step);
  const angle = valueToAngle(snappedValue, min, max);
  const fill = valueFillAngles(snappedValue, min, max, center);
  const wellSizeClass = size === "sm" ? "size-6" : "size-9";
  const faceInsetClass = size === "sm" ? "inset-[3px]" : "inset-1.5";
  const tickClass = size === "sm" ? "h-[32%] w-px" : "h-[34%] w-0.5";
  const labelClass = cn(
    "font-semibold uppercase tracking-wide",
    size === "sm" ? "text-[7px]" : "text-[8px]",
  );

  const updateFromPointer = useCallback(
    (clientY: number) => {
      const drag = dragRef.current;
      if (!drag) {
        return;
      }
      const deltaY = drag.startY - clientY;
      const range = max - min;
      const raw = drag.startValue + (deltaY / 72) * range;
      const next = Math.min(max, Math.max(min, Math.round(raw / step) * step));
      onValueChange(next);
    },
    [max, min, onValueChange, step],
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
      onValueChange(Math.min(max, value + step));
    }
    if (event.key === "ArrowDown" || event.key === "ArrowLeft") {
      event.preventDefault();
      onValueChange(Math.max(min, value - step));
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

  return (
    <div className={cn("flex flex-col items-center gap-0.5", className)}>
      <span className={cn(labelClass, accentClass ?? "text-zinc-500")}>
        {label}
      </span>
      <button
        type="button"
        disabled={disabled}
        aria-label={ariaLabel ?? `${label} EQ`}
        aria-valuemin={min}
        aria-valuemax={max}
        aria-valuenow={value}
        role="slider"
        className={cn(
          "relative touch-none rounded-full outline-none select-none",
          wellSizeClass,
          // Recessed well (no full accent ring — travel arc is drawn separately)
          "transition-[box-shadow,opacity] focus-visible:ring-2 focus-visible:ring-ring/40",
          "disabled:cursor-not-allowed disabled:opacity-45",
          "active:brightness-110",
        )}
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
        onPointerCancel={handlePointerUp}
        onKeyDown={handleKeyDown}
      >
        <TravelArcs
          fillFromDeg={fill.from}
          fillToDeg={fill.to}
          fillClassName={ringClass}
          strokeWidth={size === "sm" ? 6 : 10}
        />

        {/* Raised knob face */}
        <span
          aria-hidden
          className={cn(
            "flex justify-center absolute rounded-full bg-zinc-800",
            faceInsetClass,
          )}
          style={{
            transform: `rotate(${angle}deg)`,
          }}
        >
          <span
            aria-hidden
            className={cn("rounded-full bg-zinc-200", tickClass)}
          />
        </span>
      </button>
    </div>
  );
}
