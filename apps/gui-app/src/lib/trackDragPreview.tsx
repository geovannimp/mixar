import { createRoot } from "react-dom/client";
import { flushSync } from "react-dom";
import { TrackDragCard } from "@/components/TrackDragCard";
import type { LibraryTableRow } from "@/types";
import { writeTrackDragData } from "./libraryTable";

export function startTrackDrag(dataTransfer: DataTransfer, row: LibraryTableRow): void {
  writeTrackDragData(dataTransfer, row);

  const host = document.createElement("div");
  host.style.position = "fixed";
  host.style.top = "-1000px";
  host.style.left = "0";
  host.style.pointerEvents = "none";
  host.style.zIndex = "9999";
  document.body.appendChild(host);

  const root = createRoot(host);
  flushSync(() => {
    root.render(<TrackDragCard row={row} />);
  });

  dataTransfer.setDragImage(host, 96, 22);

  window.setTimeout(() => {
    root.unmount();
    host.remove();
  }, 0);
}
