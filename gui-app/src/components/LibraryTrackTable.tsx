import { useMemo } from "react";
import { ArrowDown, ArrowUp, ArrowUpDown } from "lucide-react";
import { DECK_LABELS } from "../lib/ui";
import {
  formatColumnValue,
  LIBRARY_TABLE_COLUMNS,
  rowKey,
  rowMatchesFilter,
  rowTrackId,
  sortLibraryRows,
} from "../lib/libraryTable";
import { startTrackDrag } from "../lib/trackDragPreview";
import type { LibraryTableColumn, LibraryTableRow } from "../types";
import { TrackActionsMenu } from "./TrackActionsMenu";

type SortDirection = "asc" | "desc";

interface LibraryTrackTableProps {
  rows: LibraryTableRow[];
  columns: LibraryTableColumn[];
  filter: string;
  sortColumn: LibraryTableColumn;
  sortDirection: SortDirection;
  emptyMessage: string;
  engineRunning: boolean;
  busy: boolean;
  analyzingTrackId: string | null;
  onSortChange: (column: LibraryTableColumn) => void;
  onLoadToDeck: (deckId: number, row: LibraryTableRow) => void;
  onAnalyze: (trackId: string) => void;
}

export function LibraryTrackTable({
  rows,
  columns,
  filter,
  sortColumn,
  sortDirection,
  emptyMessage,
  engineRunning,
  busy,
  analyzingTrackId,
  onSortChange,
  onLoadToDeck,
  onAnalyze,
}: LibraryTrackTableProps) {
  const visibleColumns = useMemo(
    () =>
      LIBRARY_TABLE_COLUMNS.filter(
        (column) => column.required || columns.includes(column.id),
      ),
    [columns],
  );

  const displayRows = useMemo(() => {
    const filtered = rows.filter((row) => rowMatchesFilter(row, filter));
    return sortLibraryRows(filtered, sortColumn, sortDirection);
  }, [rows, filter, sortColumn, sortDirection]);

  const dragEnabled = engineRunning && !busy;

  if (rows.length === 0) {
    return (
      <p className="rounded border border-dashed border-white/10 px-4 py-8 text-center text-sm text-zinc-500">
        {emptyMessage}
      </p>
    );
  }

  if (displayRows.length === 0) {
    return (
      <p className="rounded border border-dashed border-white/10 px-4 py-8 text-center text-sm text-zinc-500">
        No tracks match your filter.
      </p>
    );
  }

  return (
    <table className="w-full min-w-[40rem] border-collapse text-sm">
      <thead className="sticky top-0 z-10 bg-zinc-900/95 text-left text-[10px] font-semibold uppercase tracking-widest text-zinc-500">
        <tr className="border-b border-white/8">
          {visibleColumns.map((column) => (
            <th key={column.id} className="px-2 py-2 font-semibold">
              <button
                type="button"
                className="inline-flex items-center gap-1 transition hover:text-zinc-300"
                onClick={() => onSortChange(column.id)}
              >
                <span>{column.label}</span>
                <SortIndicator
                  active={sortColumn === column.id}
                  direction={sortDirection}
                />
              </button>
            </th>
          ))}
          <th className="w-10 px-2 py-2 text-right font-semibold" aria-label="Actions" />
        </tr>
      </thead>
      <tbody>
        {displayRows.map((row) => {
          const trackId = rowTrackId(row);
          const isAnalyzing = trackId != null && analyzingTrackId === trackId;

          return (
            <tr
              key={rowKey(row)}
              draggable={dragEnabled}
              className={
                dragEnabled
                  ? "cursor-grab border-b border-white/5 transition hover:bg-white/3 active:cursor-grabbing"
                  : "cursor-not-allowed border-b border-white/5 opacity-80"
              }
              title={dragEnabled ? "Drag to a deck" : "Start the engine to drag tracks"}
              onDragStart={(event) => {
                if (!dragEnabled) {
                  event.preventDefault();
                  return;
                }
                startTrackDrag(event.dataTransfer, row);
              }}
            >
              {visibleColumns.map((column) => (
                <td
                  key={column.id}
                  className={
                    column.id === "title"
                      ? "max-w-[10rem] truncate px-2 py-1.5 font-medium sm:max-w-xs"
                      : column.id === "path"
                        ? "max-w-[12rem] truncate px-2 py-1.5 text-zinc-500"
                        : "whitespace-nowrap px-2 py-1.5 text-zinc-300"
                  }
                  title={
                    column.id === "title" || column.id === "path"
                      ? formatColumnValue(row, column.id)
                      : undefined
                  }
                >
                  {formatColumnValue(row, column.id)}
                </td>
              ))}
              <td className="px-2 py-1.5">
                <TrackActionsMenu
                  busy={busy}
                  actions={[
                    ...DECK_LABELS.map((label, deckId) => ({
                      label: `Load to ${label}`,
                      disabled: !engineRunning,
                      onClick: () => onLoadToDeck(deckId, row),
                    })),
                    ...(rowTrackId(row)
                      ? [
                          {
                            label: isAnalyzing ? "Analyzing…" : "Analyze",
                            disabled: isAnalyzing,
                            onClick: () => {
                              const trackId = rowTrackId(row);
                              if (trackId) {
                                onAnalyze(trackId);
                              }
                            },
                          },
                        ]
                      : []),
                  ]}
                />
              </td>
            </tr>
          );
        })}
      </tbody>
    </table>
  );
}

function SortIndicator({
  active,
  direction,
}: {
  active: boolean;
  direction: SortDirection;
}) {
  if (!active) {
    return <ArrowUpDown className="size-3 opacity-40" aria-hidden />;
  }

  if (direction === "asc") {
    return <ArrowUp className="size-3 text-emerald-400" aria-hidden />;
  }

  return <ArrowDown className="size-3 text-emerald-400" aria-hidden />;
}
