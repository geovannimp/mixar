import { useEffect, useRef, useState, type ReactNode } from "react";
import { MoreHorizontal } from "lucide-react";
import { buttonIcon } from "../lib/ui";

export interface TrackActionItem {
  label: string;
  disabled?: boolean;
  onClick: () => void;
}

interface TrackActionsMenuProps {
  actions: TrackActionItem[];
  busy?: boolean;
}

export function TrackActionsMenu({ actions, busy = false }: TrackActionsMenuProps) {
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) {
      return;
    }

    const handlePointerDown = (event: PointerEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) {
        setOpen(false);
      }
    };

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setOpen(false);
      }
    };

    window.addEventListener("pointerdown", handlePointerDown);
    window.addEventListener("keydown", handleKeyDown);
    return () => {
      window.removeEventListener("pointerdown", handlePointerDown);
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [open]);

  return (
    <div ref={rootRef} className="relative flex justify-end">
      <button
        type="button"
        className={`${buttonIcon} border-white/10 bg-white/5 hover:bg-white/10`}
        disabled={busy}
        draggable={false}
        aria-label="Track actions"
        aria-expanded={open}
        onClick={() => setOpen((current) => !current)}
      >
        <MoreHorizontal className="size-4" aria-hidden />
      </button>
      {open && (
        <div
          className="absolute top-full right-0 z-20 mt-1 min-w-40 rounded-md border border-white/10 bg-zinc-900 py-1 shadow-lg"
          role="menu"
        >
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
      )}
    </div>
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
      onClick={onClick}
    >
      {children}
    </button>
  );
}
