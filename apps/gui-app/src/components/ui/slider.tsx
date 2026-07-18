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
  "group-data-[orientation=vertical]/slider:after:left-1/2 group-data-[orientation=vertical]/slider:after:top-1/2 group-data-[orientation=vertical]/slider:after:h-px group-data-[orientation=vertical]/slider:after:w-2 group-data-[orientation=vertical]/slider:after:-translate-x-1/2 group-data-[orientation=vertical]/slider:after:-translate-y-1/2 group-data-[orientation=horizontal]/slider:after:left-1/2 group-data-[orientation=horizontal]/slider:after:top-1/2 group-data-[orientation=horizontal]/slider:after:h-1.5 group-data-[orientation=horizontal]/slider:after:w-px group-data-[orientation=horizontal]/slider:after:-translate-x-1/2 group-data-[orientation=horizontal]/slider:after:-translate-y-1/2";

export function Slider({
  className,
  children,
  defaultValue,
  value,
  min = 0,
  max = 100,
  thumbAlignment = "edge",
  showIndicator = true,
  thumbVariant = "default",
  channelAccent,
  crossfaderTrack = false,
  ...props
}: SliderPrimitive.Root.Props & {
  showIndicator?: boolean;
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

  return (
    <SliderPrimitive.Root
      className={cn("group/slider data-[orientation=horizontal]:w-full", className)}
      defaultValue={defaultValue}
      max={max}
      min={min}
      thumbAlignment={thumbAlignment}
      value={value}
      {...props}
    >
      {children}
      <SliderPrimitive.Control
        className="flex touch-none select-none data-disabled:pointer-events-none data-[orientation=vertical]:h-full data-[orientation=vertical]:min-h-44 data-[orientation=horizontal]:w-full data-[orientation=horizontal]:min-w-44 data-[orientation=vertical]:flex-col data-disabled:opacity-64"
        data-slot="slider-control"
      >
        <SliderPrimitive.Track
          className={cn(
            "relative grow select-none before:absolute before:rounded-full data-[orientation=horizontal]:h-1 data-[orientation=vertical]:h-full data-[orientation=horizontal]:w-full data-[orientation=vertical]:w-1 data-[orientation=horizontal]:before:inset-x-0.5 data-[orientation=vertical]:before:inset-x-0 data-[orientation=horizontal]:before:inset-y-0 data-[orientation=vertical]:before:inset-y-0.5",
            thumbVariant === "fader"
              ? crossfaderTrack
                ? CROSSFADER_TRACK
                : channelFader.trackBg
              : "before:bg-input",
          )}
          data-slot="slider-track"
        >
          {showIndicator ? (
            <SliderPrimitive.Indicator
              className={cn(
                "select-none rounded-full data-[orientation=horizontal]:ms-0.5 data-[orientation=vertical]:mb-0.5",
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
                "relative block shrink-0 select-none outline-none transition-[box-shadow,scale] has-focus-visible:ring-[3px] has-focus-visible:ring-ring/24 dark:has-focus-visible:ring-ring/48 data-dragging:scale-105",
                thumbVariant === "fader"
                  ? cn(
                      "rounded-[2px] after:pointer-events-none after:absolute after:content-['']",
                      FADER_THUMB_SIZE,
                      FADER_GRIP_POSITION,
                      FADER_KNOB.thumb,
                      FADER_KNOB.grip,
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
