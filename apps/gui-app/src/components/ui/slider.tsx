"use client";

import { Slider as SliderPrimitive } from "@base-ui/react/slider";
import * as React from "react";
import {
  CROSSFADER_TRACK,
  DECK_ACCENTS,
  FADER_KNOB,
  NEUTRAL_FADER_TRACK,
  type DeckAccent,
} from "@/lib/ui";
import { cn } from "@/lib/utils";

const FADER_THUMB_SIZE =
  "group-data-[orientation=vertical]/slider:h-2.5 group-data-[orientation=vertical]/slider:w-5 group-data-[orientation=horizontal]/slider:h-4 group-data-[orientation=horizontal]/slider:w-2.5";

const FADER_GRIP_POSITION =
  "group-data-[orientation=vertical]/slider:after:left-1/2 group-data-[orientation=vertical]/slider:after:top-1/2 group-data-[orientation=vertical]/slider:after:h-px group-data-[orientation=vertical]/slider:after:w-2.5 group-data-[orientation=vertical]/slider:after:-translate-x-1/2 group-data-[orientation=vertical]/slider:after:-translate-y-1/2 group-data-[orientation=horizontal]/slider:after:left-1/2 group-data-[orientation=horizontal]/slider:after:top-1/2 group-data-[orientation=horizontal]/slider:after:h-2 group-data-[orientation=horizontal]/slider:after:w-px group-data-[orientation=horizontal]/slider:after:-translate-x-1/2 group-data-[orientation=horizontal]/slider:after:-translate-y-1/2";

/** Soft snap near mid — narrow so centering is optional, not sticky. */
const CENTER_SNAP_THRESHOLD = 0.8;

type TickSize = "major" | "mid" | "minor";

interface FaderTick {
  pos: number;
  size: TickSize;
}

/** Hierarchical ticks every 10%: major at ends+center, otherwise minor. */
const FADER_TICKS: FaderTick[] = Array.from({ length: 11 }, (_, i) => {
  const pos = i * 10;
  const size: TickSize = pos === 0 || pos === 50 || pos === 100 ? "major" : "minor";
  return { pos, size };
});

/** Pixel lengths for tick marks (not % of narrow fader width). */
const TICK_LENGTH: Record<TickSize, string> = {
  major: "w-2.5",
  mid: "w-2",
  minor: "w-1.5",
};

const TICK_LENGTH_H: Record<TickSize, string> = {
  major: "h-2.5",
  mid: "h-2",
  minor: "h-1.5",
};

/**
 * Decorative marker layer — pixel-sized ticks beside the track.
 * Tick centers map to 0–100% of the control (same as thumb centers).
 * Rounded lane caps extend past the control via the sibling lane element.
 */
function FaderMarkerOverlay({ centerNotch }: { centerNotch: boolean }) {
  return (
    <div
      className="pointer-events-none absolute inset-0 z-10 overflow-visible"
      aria-hidden
      data-slot="slider-markers"
    >
      {FADER_TICKS.map(({ pos, size }) => {
        const emphasize = centerNotch && pos === 50;
        const tickSize = emphasize ? "major" : size;
        const tone = emphasize ? "bg-zinc-500/25" : "bg-zinc-500/20";

        return (
          <React.Fragment key={pos}>
            {/* Vertical fader: ticks left / right of track */}
            <span
              className={cn(
                "absolute h-0.5 -translate-y-1/2 group-data-[orientation=horizontal]/slider:hidden",
                "right-[calc(50%+5px)]",
                tone,
                TICK_LENGTH[tickSize],
              )}
              style={{ top: `${pos}%` }}
            />
            <span
              className={cn(
                "absolute h-0.5 -translate-y-1/2 group-data-[orientation=horizontal]/slider:hidden",
                "left-[calc(50%+5px)]",
                tone,
                TICK_LENGTH[tickSize],
              )}
              style={{ top: `${pos}%` }}
            />
            {/* Horizontal fader: ticks above / below track */}
            <span
              className={cn(
                "absolute w-0.5 -translate-x-1/2 group-data-[orientation=vertical]/slider:hidden",
                "bottom-[calc(50%+5px)]",
                tone,
                TICK_LENGTH_H[tickSize],
              )}
              style={{ left: `${pos}%` }}
            />
            <span
              className={cn(
                "absolute w-0.5 -translate-x-1/2 group-data-[orientation=vertical]/slider:hidden",
                "top-[calc(50%+5px)]",
                tone,
                TICK_LENGTH_H[tickSize],
              )}
              style={{ left: `${pos}%` }}
            />
          </React.Fragment>
        );
      })}

      {centerNotch ? (
        <>
          <span
            className={cn(
              "absolute left-1/2 top-1/2 h-0.5 w-3.5 -translate-x-1/2 -translate-y-1/2 rounded-sm bg-zinc-500/20",
              "group-data-[orientation=horizontal]/slider:hidden",
            )}
            data-slot="slider-center-notch"
          />
          <span
            className={cn(
              "absolute left-1/2 top-1/2 h-3.5 w-0.5 -translate-x-1/2 -translate-y-1/2 rounded-sm bg-zinc-500/20",
              "group-data-[orientation=vertical]/slider:hidden",
            )}
            data-slot="slider-center-notch"
          />
        </>
      ) : null}
    </div>
  );
}

function snapTowardCenter(value: number, min: number, max: number): number {
  const mid = (min + max) / 2;
  return Math.abs(value - mid) <= CENTER_SNAP_THRESHOLD ? mid : value;
}

function normalizeSliderValue(value: number | readonly number[]): number {
  return typeof value === "number" ? value : (value[0] ?? 0);
}

export function Slider({
  className,
  children,
  defaultValue,
  value,
  min = 0,
  max = 100,
  thumbAlignment = "edge",
  showIndicator = true,
  showMarkers = false,
  centerNotch = false,
  thumbVariant = "default",
  channelAccent,
  crossfaderTrack = false,
  onValueChange,
  onValueCommitted,
  step = 1,
  ...props
}: SliderPrimitive.Root.Props & {
  showIndicator?: boolean;
  showMarkers?: boolean;
  centerNotch?: boolean;
  thumbVariant?: "default" | "fader";
  channelAccent?: DeckAccent;
  crossfaderTrack?: boolean;
}): React.ReactElement {
  const _values = React.useMemo(() => {
    if (value !== undefined) {
      return Array.isArray(value) ? value : [value];
    }
    if (defaultValue !== undefined) {
      return Array.isArray(defaultValue) ? defaultValue : [defaultValue];
    }
    return [min];
  }, [value, defaultValue, min]);

  const channelFader = channelAccent ? DECK_ACCENTS[channelAccent].fader : NEUTRAL_FADER_TRACK;
  const faderLane =
    thumbVariant === "fader" ? (crossfaderTrack ? CROSSFADER_TRACK : channelFader.trackBg) : null;
  const markedFader = thumbVariant === "fader" && showMarkers;

  const handleValueChange: SliderPrimitive.Root.Props["onValueChange"] = (next, eventDetails) => {
    if (!onValueChange) {
      return;
    }
    if (!centerNotch) {
      onValueChange(next, eventDetails);
      return;
    }
    const raw = normalizeSliderValue(next);
    const snapped = snapTowardCenter(raw, min, max);
    if (Array.isArray(next)) {
      onValueChange([snapped, ...next.slice(1)] as typeof next, eventDetails);
      return;
    }
    onValueChange(snapped, eventDetails);
  };

  return (
    <SliderPrimitive.Root
      className={cn("group/slider relative data-[orientation=horizontal]:w-full", className)}
      defaultValue={defaultValue}
      max={max}
      min={min}
      step={step}
      thumbAlignment={thumbAlignment}
      value={value}
      onValueChange={handleValueChange}
      onValueCommitted={onValueCommitted}
      {...props}
    >
      {children}
      <SliderPrimitive.Control
        className="relative z-1 flex touch-none select-none overflow-visible data-disabled:pointer-events-none data-[orientation=vertical]:h-full data-[orientation=vertical]:min-h-44 data-[orientation=horizontal]:w-full data-[orientation=horizontal]:min-w-44 data-[orientation=vertical]:flex-col data-disabled:opacity-64"
        data-slot="slider-control"
      >
        {markedFader ? (
          <div
            aria-hidden
            className={cn(
              "pointer-events-none absolute z-0 rounded-full before:absolute before:inset-0 before:rounded-full",
              // Full control span + slight extension past end markers for rounded caps.
              "group-data-[orientation=vertical]/slider:left-1/2 group-data-[orientation=vertical]/slider:w-1 group-data-[orientation=vertical]/slider:-translate-x-1/2",
              "group-data-[orientation=vertical]/slider:-top-1.5 group-data-[orientation=vertical]/slider:-bottom-1.5",
              "group-data-[orientation=horizontal]/slider:top-1/2 group-data-[orientation=horizontal]/slider:h-1 group-data-[orientation=horizontal]/slider:-translate-y-1/2",
              "group-data-[orientation=horizontal]/slider:-left-1.5 group-data-[orientation=horizontal]/slider:-right-1.5",
              faderLane,
            )}
            data-slot="slider-lane"
          />
        ) : null}
        {showMarkers ? <FaderMarkerOverlay centerNotch={centerNotch} /> : null}
        <SliderPrimitive.Track
          className={cn(
            "relative grow select-none before:absolute before:z-1 before:rounded-full data-[orientation=horizontal]:h-1 data-[orientation=vertical]:h-full data-[orientation=horizontal]:w-full data-[orientation=vertical]:w-1",
            markedFader
              ? "before:hidden"
              : thumbVariant === "fader"
                ? cn(
                    "data-[orientation=horizontal]:before:inset-x-0 data-[orientation=vertical]:before:inset-x-0 data-[orientation=horizontal]:before:inset-y-0 data-[orientation=vertical]:before:inset-y-0",
                    faderLane,
                  )
                : "before:bg-input data-[orientation=horizontal]:before:inset-x-0.5 data-[orientation=vertical]:before:inset-x-0 data-[orientation=horizontal]:before:inset-y-0 data-[orientation=vertical]:before:inset-y-0.5",
          )}
          data-slot="slider-track"
        >
          {showIndicator ? (
            <SliderPrimitive.Indicator
              className={cn(
                "relative z-1 select-none rounded-full data-[orientation=horizontal]:ms-0.5 data-[orientation=vertical]:mb-0.5",
                thumbVariant === "fader" && channelAccent
                  ? DECK_ACCENTS[channelAccent].fader.indicator
                  : "bg-primary",
              )}
              data-slot="slider-indicator"
            />
          ) : null}
          {Array.from({ length: _values.length }, (_, index) => (
            <SliderPrimitive.Thumb
              className={cn(
                "relative z-2 block shrink-0 select-none outline-none transition-[box-shadow,scale] has-focus-visible:ring-[3px] has-focus-visible:ring-ring/24 dark:has-focus-visible:ring-ring/48 data-dragging:scale-105",
                thumbVariant === "fader"
                  ? cn(
                      "rounded-[2px] after:pointer-events-none after:absolute after:content-['']",
                      FADER_THUMB_SIZE,
                      FADER_GRIP_POSITION,
                      FADER_KNOB.thumb,
                      channelFader.grip,
                      FADER_KNOB.focusRing,
                    )
                  : "size-5 rounded-full border border-input bg-white not-dark:bg-clip-padding shadow-xs/5 before:absolute before:inset-0 before:rounded-full before:shadow-[0_1px_--theme(--color-black/4%)] sm:size-4 dark:border-background [:has(*:focus-visible),[data-dragging]]:shadow-none",
              )}
              data-slot="slider-thumb"
              index={index}
              key={String(index)}
            />
          ))}
        </SliderPrimitive.Track>
      </SliderPrimitive.Control>
    </SliderPrimitive.Root>
  );
}

export function SliderValue({
  className,
  ...props
}: SliderPrimitive.Value.Props): React.ReactElement {
  return (
    <SliderPrimitive.Value
      className={cn("flex justify-end text-sm", className)}
      data-slot="slider-value"
      {...props}
    />
  );
}

export { SliderPrimitive };
