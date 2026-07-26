import { useEffect, useMemo, useState, type ReactNode } from "react";
import { pointerIntersection } from "@dnd-kit/collision";
import { useDroppable } from "@dnd-kit/react";
import { registerOsFileDropTarget, updateOsFileDropTarget } from "@/lib/osFileDropTargets";
import { isTauriApp } from "@/lib/tauriApp";
import { DROP_HIGHLIGHT_CLASS, TRACK_DRAG_TYPE, type TrackDropData } from "@/lib/trackDrag";
import { cn } from "@/lib/utils";

type DropState = {
  isOver: boolean;
};

interface TrackDropZoneProps {
  id: string;
  data: TrackDropData;
  disabled?: boolean;
  /** Sampler pads should be higher than the deck so nested pads win. */
  collisionPriority?: number;
  className?: string | ((state: DropState) => string);
  children: ReactNode | ((state: DropState) => ReactNode);
}

/**
 * dnd-kit droppable for in-app track drags, plus registration for Tauri OS file drops.
 * OS files cannot be reliable dnd-kit sources (programmatic start/collision is fragile);
 * they hit the same zones via OsFileDropBridge.
 */
export function TrackDropZone({
  id,
  data,
  disabled = false,
  collisionPriority = 0,
  className,
  children,
}: TrackDropZoneProps) {
  const [node, setNode] = useState<HTMLElement | null>(null);
  const [osHover, setOsHover] = useState(false);
  const tauri = isTauriApp();

  const { ref: droppableRef, isDropTarget } = useDroppable({
    id,
    data,
    disabled,
    accept: TRACK_DRAG_TYPE,
    collisionDetector: pointerIntersection,
    collisionPriority,
  });

  useEffect(() => {
    if (!tauri || !node) {
      return;
    }
    return registerOsFileDropTarget({
      id,
      element: node,
      data,
      collisionPriority,
      disabled,
      setHover: setOsHover,
    });
  }, [tauri, node, id, data, collisionPriority, disabled]);

  useEffect(() => {
    if (!tauri) {
      return;
    }
    updateOsFileDropTarget(id, { disabled, collisionPriority, data });
  }, [tauri, id, disabled, collisionPriority, data]);

  const isOver = isDropTarget || osHover;
  const state = useMemo<DropState>(() => ({ isOver }), [isOver]);
  const resolvedClassName = typeof className === "function" ? className(state) : className;

  return (
    <div
      data-track-drop-zone={id}
      ref={(element) => {
        droppableRef(element);
        setNode(element instanceof HTMLElement ? element : null);
      }}
      className={cn(resolvedClassName, isOver && DROP_HIGHLIGHT_CLASS)}
    >
      {typeof children === "function" ? children(state) : children}
    </div>
  );
}
