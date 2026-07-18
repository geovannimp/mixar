import {
  flexRender,
  getCoreRowModel,
  getFilteredRowModel,
  getSortedRowModel,
  useReactTable,
  type ColumnDef,
  type Row,
  type SortingState,
  type Table,
} from "@tanstack/react-table";
import { useVirtualizer, type VirtualItem } from "@tanstack/react-virtual";
import { ArrowDown, ArrowUp, ArrowUpDown } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { Spinner } from "@/components/ui/spinner";
import { cn } from "@/lib/utils";
import { DECK_LABELS } from "@/lib/ui";
import {
  columnSortValue,
  formatColumnValue,
  LIBRARY_TABLE_COLUMNS,
  libraryRowSearchText,
  rowKey,
  rowTitle,
  rowTrackId,
} from "@/lib/libraryTable";
import { fuzzyFilter, fuzzySort, libraryGlobalFilter } from "@/lib/libraryTableFilter";
import { startTrackDrag } from "@/lib/trackDragPreview";
import type { LibraryTableColumn, LibraryTableRow } from "@/types";
import { TrackActionsMenu } from "./TrackActionsMenu";

const SEARCH_COLUMN_ID = "searchText";
const ROW_HEIGHT = 36;

interface LibraryTrackTableProps {
  rows: LibraryTableRow[];
  columns: LibraryTableColumn[];
  globalFilter: string;
  emptyMessage: string;
  engineRunning: boolean;
  busy: boolean;
  analyzingTrackId: string | null;
  onLoadToDeck: (deckId: number, row: LibraryTableRow) => void;
  onAnalyze: (trackId: string) => void;
}

export function LibraryTrackTable({
  rows,
  columns,
  globalFilter,
  emptyMessage,
  engineRunning,
  busy,
  analyzingTrackId,
  onLoadToDeck,
  onAnalyze,
}: LibraryTrackTableProps) {
  const tableContainerRef = useRef<HTMLDivElement>(null);
  const [sorting, setSorting] = useState<SortingState>([]);

  const visibleColumns = useMemo(
    () => LIBRARY_TABLE_COLUMNS.filter((column) => column.required || columns.includes(column.id)),
    [columns],
  );

  const columnDefs = useMemo<ColumnDef<LibraryTableRow>[]>(() => {
    const dataColumns: ColumnDef<LibraryTableRow>[] = visibleColumns.map((column) => ({
      id: column.id,
      accessorFn: (row) => columnSortValue(row, column.id),
      header: column.label,
      cell: ({ row }) => row.original,
      sortingFn: "alphanumeric",
      meta: { columnId: column.id },
    }));

    return [
      {
        id: SEARCH_COLUMN_ID,
        accessorFn: libraryRowSearchText,
        filterFn: "fuzzy",
        sortingFn: fuzzySort,
        enableSorting: false,
      },
      ...dataColumns,
      {
        id: "actions",
        enableSorting: false,
        header: () => null,
        cell: ({ row }) => row.original,
        meta: { columnId: "actions" as const },
      },
    ];
  }, [visibleColumns]);

  const table = useReactTable({
    data: rows,
    columns: columnDefs,
    filterFns: {
      fuzzy: fuzzyFilter,
    },
    state: {
      globalFilter,
      sorting,
      columnVisibility: {
        [SEARCH_COLUMN_ID]: false,
      },
    },
    onSortingChange: setSorting,
    globalFilterFn: libraryGlobalFilter,
    getCoreRowModel: getCoreRowModel(),
    getFilteredRowModel: getFilteredRowModel(),
    getSortedRowModel: getSortedRowModel(),
    enableSortingRemoval: true,
    getRowId: (row) => rowKey(row),
  });

  useEffect(() => {
    if (globalFilter.trim()) {
      setSorting((current) =>
        current[0]?.id === SEARCH_COLUMN_ID ? current : [{ id: SEARCH_COLUMN_ID, desc: false }],
      );
      return;
    }

    setSorting((current) => (current[0]?.id === SEARCH_COLUMN_ID ? [] : current));
  }, [globalFilter]);

  const dragEnabled = engineRunning && !busy;
  const filteredRows = table.getRowModel().rows;

  if (rows.length === 0) {
    return (
      <p className="rounded border border-dashed border-white/10 px-4 py-8 text-center text-sm text-zinc-500">
        {emptyMessage}
      </p>
    );
  }

  if (filteredRows.length === 0) {
    return (
      <p className="rounded border border-dashed border-white/10 px-4 py-8 text-center text-sm text-zinc-500">
        No tracks match your filter.
      </p>
    );
  }

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div ref={tableContainerRef} className="min-h-0 flex-1 overflow-auto">
        <table className="w-full min-w-[40rem] border-collapse text-sm">
          <thead className="sticky top-0 z-10 bg-zinc-900/95 text-left text-[10px] font-semibold uppercase tracking-widest text-zinc-500">
            {table.getHeaderGroups().map((headerGroup) => (
              <tr key={headerGroup.id} className="flex w-full border-b border-white/8">
                {headerGroup.headers.map((header) => {
                  if (header.column.id === SEARCH_COLUMN_ID) {
                    return null;
                  }

                  const columnId = header.column.id as LibraryTableColumn | "actions";
                  const canSort = header.column.getCanSort();

                  return (
                    <th
                      key={header.id}
                      className={cn(
                        "px-2 py-2 font-semibold",
                        columnCellClass(columnId),
                        columnId === "actions" && "text-right",
                      )}
                      aria-label={columnId === "actions" ? "Actions" : undefined}
                    >
                      {canSort ? (
                        <button
                          type="button"
                          className="inline-flex items-center gap-1 transition hover:text-zinc-300"
                          onClick={header.column.getToggleSortingHandler()}
                        >
                          <span>
                            {flexRender(header.column.columnDef.header, header.getContext())}
                          </span>
                          <SortIndicator direction={header.column.getIsSorted()} />
                        </button>
                      ) : columnId === "actions" ? null : (
                        flexRender(header.column.columnDef.header, header.getContext())
                      )}
                    </th>
                  );
                })}
              </tr>
            ))}
          </thead>
          <LibraryTrackTableBody
            table={table}
            tableContainerRef={tableContainerRef}
            visibleColumns={visibleColumns}
            dragEnabled={dragEnabled}
            analyzingTrackId={analyzingTrackId}
            busy={busy}
            engineRunning={engineRunning}
            onLoadToDeck={onLoadToDeck}
            onAnalyze={onAnalyze}
          />
        </table>
      </div>
    </div>
  );
}

interface LibraryTrackTableBodyProps {
  table: Table<LibraryTableRow>;
  tableContainerRef: React.RefObject<HTMLDivElement | null>;
  visibleColumns: { id: LibraryTableColumn; label: string }[];
  dragEnabled: boolean;
  analyzingTrackId: string | null;
  busy: boolean;
  engineRunning: boolean;
  onLoadToDeck: (deckId: number, row: LibraryTableRow) => void;
  onAnalyze: (trackId: string) => void;
}

function LibraryTrackTableBody({
  table,
  tableContainerRef,
  visibleColumns,
  dragEnabled,
  analyzingTrackId,
  busy,
  engineRunning,
  onLoadToDeck,
  onAnalyze,
}: LibraryTrackTableBodyProps) {
  const { rows } = table.getRowModel();

  const rowVirtualizer = useVirtualizer({
    count: rows.length,
    estimateSize: () => ROW_HEIGHT,
    getScrollElement: () => tableContainerRef.current,
    overscan: 8,
  });

  return (
    <tbody className="relative block" style={{ height: `${rowVirtualizer.getTotalSize()}px` }}>
      {rowVirtualizer.getVirtualItems().map((virtualRow) => {
        const row = rows[virtualRow.index];
        if (!row) {
          return null;
        }

        return (
          <LibraryTrackTableRow
            key={row.id}
            row={row}
            virtualRow={virtualRow}
            visibleColumns={visibleColumns}
            dragEnabled={dragEnabled}
            analyzingTrackId={analyzingTrackId}
            busy={busy}
            engineRunning={engineRunning}
            onLoadToDeck={onLoadToDeck}
            onAnalyze={onAnalyze}
          />
        );
      })}
    </tbody>
  );
}

interface LibraryTrackTableRowProps {
  row: Row<LibraryTableRow>;
  virtualRow: VirtualItem;
  visibleColumns: { id: LibraryTableColumn; label: string }[];
  dragEnabled: boolean;
  analyzingTrackId: string | null;
  busy: boolean;
  engineRunning: boolean;
  onLoadToDeck: (deckId: number, row: LibraryTableRow) => void;
  onAnalyze: (trackId: string) => void;
}

function LibraryTrackTableRow({
  row,
  virtualRow,
  visibleColumns,
  dragEnabled,
  analyzingTrackId,
  busy,
  engineRunning,
  onLoadToDeck,
  onAnalyze,
}: LibraryTrackTableRowProps) {
  const tableRow = row.original;
  const trackId = rowTrackId(tableRow);
  const isAnalyzing = trackId != null && analyzingTrackId === trackId;

  return (
    <tr
      draggable={dragEnabled && !isAnalyzing}
      aria-busy={isAnalyzing}
      className={cn(
        "absolute flex w-full border-b border-white/5 transition",
        !isAnalyzing &&
          (dragEnabled
            ? "cursor-grab hover:bg-white/3 active:cursor-grabbing"
            : "cursor-not-allowed opacity-80"),
      )}
      style={{
        height: `${virtualRow.size}px`,
        transform: `translateY(${virtualRow.start}px)`,
      }}
      title={
        isAnalyzing
          ? "Analyzing track…"
          : dragEnabled
            ? "Drag to a deck"
            : "Start the engine to drag tracks"
      }
      onDragStart={(event) => {
        if (!dragEnabled || isAnalyzing) {
          event.preventDefault();
          return;
        }
        startTrackDrag(event.dataTransfer, tableRow);
      }}
    >
      {visibleColumns.map((column) => (
        <td
          key={column.id}
          className={cn(
            "select-text px-2 py-1.5 text-zinc-300",
            columnCellClass(column.id),
            column.id === "title" && "font-medium text-zinc-100",
            column.id === "path" && "text-zinc-500",
          )}
          title={
            column.id === "title" || column.id === "path"
              ? isAnalyzing && column.id === "title"
                ? rowTitle(tableRow)
                : formatColumnValue(tableRow, column.id)
              : undefined
          }
        >
          {column.id === "title" && isAnalyzing ? (
            <span className="inline-flex min-w-0 items-center gap-2 text-emerald-200">
              <Spinner className="size-3.5 shrink-0 text-emerald-400" />
              <span className="truncate">{rowTitle(tableRow)}</span>
            </span>
          ) : isAnalyzing && (column.id === "bpm" || column.id === "key") ? (
            <span className="text-emerald-400/70">…</span>
          ) : (
            formatColumnValue(tableRow, column.id)
          )}
        </td>
      ))}
      <td className={cn("px-2 py-1.5", columnCellClass("actions"))}>
        <TrackActionsMenu
          busy={busy || isAnalyzing}
          actions={[
            ...DECK_LABELS.map((label, deckId) => ({
              label: `Load to ${label}`,
              disabled: !engineRunning,
              onClick: () => onLoadToDeck(deckId, tableRow),
            })),
            ...(trackId
              ? [
                  {
                    label: isAnalyzing ? "Analyzing…" : "Analyze",
                    disabled: isAnalyzing,
                    onClick: () => onAnalyze(trackId),
                  },
                ]
              : []),
          ]}
        />
      </td>
    </tr>
  );
}

function columnCellClass(columnId: LibraryTableColumn | "actions"): string {
  switch (columnId) {
    case "title":
      return "min-w-0 flex-1 basis-48 sm:basis-64 truncate";
    case "path":
      return "min-w-0 flex-1 basis-48 truncate";
    case "actions":
      return "w-10 shrink-0";
    default:
      return "shrink-0 basis-20 whitespace-nowrap";
  }
}

function SortIndicator({ direction }: { direction: false | "asc" | "desc" }) {
  if (!direction) {
    return <ArrowUpDown className="size-3 opacity-40" aria-hidden />;
  }

  if (direction === "asc") {
    return <ArrowUp className="size-3 text-emerald-400" aria-hidden />;
  }

  return <ArrowDown className="size-3 text-emerald-400" aria-hidden />;
}
