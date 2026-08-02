import { useState, type ReactNode } from "react";
import { MoreHorizontal } from "lucide-react";
import { Popover, PopoverPopup, PopoverTrigger } from "@/components/ui/popover";
import { cn } from "@/lib/utils";
import { buttonIcon } from "@/lib/ui";

export interface TrackActionItem {
  label: string;
  disabled?: boolean;
  onClick: () => void;
}

interface TrackActionsMenuProps {
  actions: TrackActionItem[];
  busy?: boolean;
  hiddenUntilHover?: boolean;
  menuLabel?: string;
}

export function TrackActionsMenu({
  actions,
  busy = false,
  hiddenUntilHover = false,
  menuLabel = "Track actions",
}: TrackActionsMenuProps) {
  const [open, setOpen] = useState(false);

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <div
        className={cn(
          "relative flex shrink-0 justify-end",
          hiddenUntilHover &&
            "opacity-0 transition-opacity group-hover:opacity-100 group-focus-within:opacity-100",
          open && "opacity-100",
        )}
      >
        <PopoverTrigger
          type="button"
          className={`${buttonIcon} border-white/10 bg-white/5 hover:bg-white/10`}
          disabled={busy}
          draggable={false}
          aria-label={menuLabel}
          onClick={(event) => {
            event.stopPropagation();
          }}
        >
          <MoreHorizontal className="size-4" aria-hidden />
        </PopoverTrigger>
        <PopoverPopup
          side="bottom"
          align="end"
          sideOffset={4}
          tooltipStyle
          className="z-50 min-w-40 rounded-md border-white/10 py-1 shadow-lg"
        >
          <div role="menu" className="flex flex-col py-0.5">
            {actions.map((action) => (
              <MenuItem
                key={action.label}
                disabled={busy || action.disabled}
                onClick={() => {
                  setOpen(false);
                  action.onClick();
                }}
              >
                {action.label}
              </MenuItem>
            ))}
          </div>
        </PopoverPopup>
      </div>
    </Popover>
  );
}

function MenuItem({
  children,
  disabled,
  onClick,
}: {
  children: ReactNode;
  disabled?: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      role="menuitem"
      className="block w-full px-3 py-1.5 text-left text-sm text-zinc-200 transition hover:bg-white/8 disabled:cursor-not-allowed disabled:opacity-45"
      disabled={disabled}
      onClick={(event) => {
        event.stopPropagation();
        onClick();
      }}
    >
      {children}
    </button>
  );
}
