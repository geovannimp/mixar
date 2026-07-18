import { Button as ButtonPrimitive } from "@base-ui/react/button";
import { cva, type VariantProps } from "class-variance-authority";

import { cn } from "@/lib/utils";
import { deckButtonAccentTone, type DeckButtonAccent } from "@/lib/ui";

const NEUTRAL_TONE = "border-white/10 bg-black/30 text-zinc-400 hover:bg-black/45";

const ACTIVE_TONE =
  "border-emerald-500/50 bg-emerald-500/15 text-emerald-200 hover:bg-emerald-500/15";

const deckButtonVariants = cva(
  "inline-flex shrink-0 items-center justify-center border transition outline-none select-none disabled:cursor-not-allowed disabled:border-white/10 disabled:bg-black/25 disabled:text-zinc-500 disabled:hover:bg-black/25",
  {
    variants: {
      size: {
        compact: "rounded px-2.5 py-1 text-xs font-medium",
        toggle: "rounded px-1.5 py-0.5 text-[10px] font-bold uppercase tracking-wider",
        cell: "h-8 min-w-0 rounded px-0 py-0 text-xs font-semibold sm:h-9",
        cellWide:
          "h-8 w-full rounded px-1 py-0 text-[9px] font-bold uppercase tracking-[0.2em] sm:h-9",
        pad: "min-h-11 flex-col rounded-md px-1 py-1.5 text-center sm:min-h-12",
        icon: "size-7 rounded text-sm font-semibold leading-none",
        sync: "mt-0.5 w-full rounded px-1 py-1 text-[9px] font-bold uppercase tracking-[0.2em]",
        circular: "size-11 rounded-full border-2 text-sm font-bold shadow-md sm:size-12",
      },
    },
    defaultVariants: {
      size: "compact",
    },
  },
);

function resolveTone({
  disabled,
  active,
  accent,
}: {
  disabled?: boolean;
  active?: boolean;
  accent?: DeckButtonAccent;
}): string {
  if (disabled) {
    return "";
  }
  if (active) {
    return ACTIVE_TONE;
  }
  if (accent !== undefined) {
    return deckButtonAccentTone(accent);
  }
  return NEUTRAL_TONE;
}

type DeckButtonProps = ButtonPrimitive.Props &
  VariantProps<typeof deckButtonVariants> & {
    active?: boolean;
    accent?: DeckButtonAccent;
  };

function DeckButton({
  className,
  active = false,
  accent,
  disabled,
  size = "compact",
  ...props
}: DeckButtonProps) {
  return (
    <ButtonPrimitive
      data-slot="deck-button"
      disabled={disabled}
      className={cn(
        deckButtonVariants({ size }),
        resolveTone({ disabled, active, accent }),
        className,
      )}
      {...props}
    />
  );
}

export { DeckButton, deckButtonVariants };
