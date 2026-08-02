import type { ReactNode } from "react";
import { DragDropProvider, DragOverlay } from "@dnd-kit/react";
import { OsFileDropBridge } from "@/components/dnd/os-file-drop-bridge";
import { TrackDragCard } from "@/components/dnd/track-drag-card";
import { applyTrackDrop, isTrackDragData, isTrackDropData } from "@/lib/track-drag";

interface TrackDragProviderProps {
  children: ReactNode;
}

export function TrackDragProvider({ children }: TrackDragProviderProps) {
  return (
    <DragDropProvider
      onDragEnd={(event) => {
        if (event.canceled) {
          return;
        }
        const source = event.operation.source;
        const target = event.operation.target;
        if (!source || !target) {
          return;
        }
        if (!isTrackDragData(source.data) || !isTrackDropData(target.data)) {
          return;
        }
        applyTrackDrop(target.data, source.data.payload);
      }}
    >
      <OsFileDropBridge />
      {children}
      <DragOverlay dropAnimation={null}>
        {(source) => {
          if (!isTrackDragData(source.data)) {
            return null;
          }
          return <TrackDragCard row={source.data.row} />;
        }}
      </DragOverlay>
    </DragDropProvider>
  );
}
