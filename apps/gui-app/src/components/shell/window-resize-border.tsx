import { useEffect, useState, type MouseEvent, type ReactNode } from "react";
import { cn } from "@/lib/utils";
import { getAppWindow, isTauriApp } from "@/lib/tauri-app";

type ResizeDirection =
  | "East"
  | "North"
  | "NorthEast"
  | "NorthWest"
  | "South"
  | "SouthEast"
  | "SouthWest"
  | "West";

const HANDLES: {
  direction: ResizeDirection;
  className: string;
  cursor: string;
}[] = [
  {
    direction: "North",
    className: "top-0 right-2.5 left-2.5 h-[5px]",
    cursor: "cursor-n-resize",
  },
  {
    direction: "South",
    className: "bottom-0 right-2.5 left-2.5 h-[5px]",
    cursor: "cursor-s-resize",
  },
  {
    direction: "West",
    className: "top-2.5 bottom-2.5 left-0 w-[5px]",
    cursor: "cursor-w-resize",
  },
  {
    direction: "East",
    className: "top-2.5 bottom-2.5 right-0 w-[5px]",
    cursor: "cursor-e-resize",
  },
  {
    direction: "NorthWest",
    className: "top-0 left-0 size-2.5",
    cursor: "cursor-nw-resize",
  },
  {
    direction: "NorthEast",
    className: "top-0 right-0 size-2.5",
    cursor: "cursor-ne-resize",
  },
  {
    direction: "SouthWest",
    className: "bottom-0 left-0 size-2.5",
    cursor: "cursor-sw-resize",
  },
  {
    direction: "SouthEast",
    className: "bottom-0 right-0 size-2.5",
    cursor: "cursor-se-resize",
  },
];

interface WindowResizeBorderProps {
  children: ReactNode;
  className?: string;
}

function WindowResizeHandles() {
  const [isMaximized, setIsMaximized] = useState(false);

  useEffect(() => {
    const appWindow = getAppWindow();
    let disposed = false;

    const syncMaximized = async () => {
      const maximized = await appWindow.isMaximized();
      if (!disposed) {
        setIsMaximized(maximized);
      }
    };

    void syncMaximized();

    const unlistenPromise = appWindow.onResized(() => {
      void syncMaximized();
    });

    return () => {
      disposed = true;
      void unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  if (isMaximized) {
    return null;
  }

  const handleMouseDown = (direction: ResizeDirection) => (event: MouseEvent<HTMLDivElement>) => {
    if (event.buttons !== 1) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();
    void getAppWindow().startResizeDragging(direction);
  };

  return (
    <div className="pointer-events-none fixed inset-0 z-200" aria-hidden>
      {HANDLES.map(({ direction, className, cursor }) => (
        <div
          key={direction}
          className={cn("pointer-events-auto absolute", className, cursor)}
          onMouseDown={handleMouseDown(direction)}
        />
      ))}
    </div>
  );
}

export function WindowResizeBorder({ children, className }: WindowResizeBorderProps) {
  if (!isTauriApp()) {
    return <div className={className}>{children}</div>;
  }

  return (
    <div
      className={cn(
        "relative flex h-screen min-h-0 flex-col overflow-hidden border border-white/8",
        className,
      )}
    >
      {children}
      <WindowResizeHandles />
    </div>
  );
}
