import {
  formatBpm,
  formatDuration,
  formatOptional,
} from "./format";
import type {
  FsEntry,
  LibraryTableColumn,
  LibraryTableRow,
  TrackSummary,
} from "../types";

export const LIBRARY_TABLE_COLUMNS: {
  id: LibraryTableColumn;
  label: string;
  required?: boolean;
}[] = [
  { id: "title", label: "Title", required: true },
  { id: "artist", label: "Artist" },
  { id: "album", label: "Album" },
  { id: "genre", label: "Genre" },
  { id: "bpm", label: "BPM" },
  { id: "key", label: "Key" },
  { id: "duration", label: "Length" },
  { id: "path", label: "Path" },
];

export const DEFAULT_LIBRARY_TABLE_COLUMNS: LibraryTableColumn[] = [
  "title",
  "artist",
  "bpm",
  "key",
  "duration",
];

export const TRACK_DRAG_MIME = "application/x-rust-dj-track";

export interface TrackDragPayload {
  source: "library" | "filesystem";
  trackId: string | null;
  path: string;
  title: string;
}

export function libraryRowFromTrack(track: TrackSummary): LibraryTableRow {
  return { source: "library", track };
}

export function libraryRowFromFile(
  file: FsEntry,
  libraryTrack?: TrackSummary,
): LibraryTableRow {
  return libraryTrack
    ? { source: "filesystem", file, libraryTrack }
    : { source: "filesystem", file };
}

export function rowKey(row: LibraryTableRow): string {
  return row.source === "library" ? row.track.id : row.file.path;
}

export function rowTitle(row: LibraryTableRow): string {
  if (row.source === "library") {
    return row.track.title?.trim() || row.track.display_name;
  }
  if (row.libraryTrack) {
    return row.libraryTrack.title?.trim() || row.libraryTrack.display_name;
  }
  return row.file.name;
}

export function rowPath(row: LibraryTableRow): string {
  return row.source === "library" ? row.track.path : row.file.path;
}

export function rowArtist(row: LibraryTableRow): string | null {
  if (row.source === "library") {
    return row.track.artist;
  }
  return row.libraryTrack?.artist ?? null;
}

export function rowMusicalKey(row: LibraryTableRow): string | null {
  if (row.source === "library") {
    return row.track.key;
  }
  return row.libraryTrack?.key ?? null;
}

export function rowBpmValue(row: LibraryTableRow): number | null {
  if (row.source === "library") {
    return row.track.bpm;
  }
  return row.libraryTrack?.bpm ?? null;
}

export function rowTrackId(row: LibraryTableRow): string | null {
  if (row.source === "library") {
    return row.track.id;
  }
  return row.libraryTrack?.id ?? null;
}

export function rowToDragPayload(row: LibraryTableRow): TrackDragPayload {
  const hasLibraryTrack = row.source === "filesystem" && row.libraryTrack;
  return {
    source: row.source === "library" || hasLibraryTrack ? "library" : "filesystem",
    trackId: rowTrackId(row),
    path: rowPath(row),
    title: rowTitle(row),
  };
}

function filesystemMetadata(row: Extract<LibraryTableRow, { source: "filesystem" }>) {
  return row.libraryTrack;
}

export function parseTrackDragPayload(
  data: string,
): TrackDragPayload | null {
  if (!data) {
    return null;
  }
  try {
    const parsed = JSON.parse(data) as TrackDragPayload;
    if (typeof parsed.path !== "string" || typeof parsed.title !== "string") {
      return null;
    }
    if (parsed.source !== "library" && parsed.source !== "filesystem") {
      return null;
    }
    return parsed;
  } catch {
    return null;
  }
}

export function writeTrackDragData(
  dataTransfer: DataTransfer,
  row: LibraryTableRow,
): void {
  const payload = JSON.stringify(rowToDragPayload(row));
  dataTransfer.setData(TRACK_DRAG_MIME, payload);
  dataTransfer.setData("text/plain", payload);
  dataTransfer.effectAllowed = "copy";
}

export function readTrackDragData(
  dataTransfer: DataTransfer,
): TrackDragPayload | null {
  const raw =
    dataTransfer.getData(TRACK_DRAG_MIME) ||
    dataTransfer.getData("text/plain");
  return parseTrackDragPayload(raw);
}

export function acceptsTrackDrag(dataTransfer: DataTransfer): boolean {
  const types = Array.from(dataTransfer.types);
  if (types.length === 0) {
    return true;
  }
  return types.includes(TRACK_DRAG_MIME) || types.includes("text/plain");
}

type SortDirection = "asc" | "desc";

function compareValues(
  left: string | number | null,
  right: string | number | null,
  direction: SortDirection,
): number {
  const factor = direction === "asc" ? 1 : -1;

  if (left == null && right == null) {
    return 0;
  }
  if (left == null) {
    return 1;
  }
  if (right == null) {
    return -1;
  }

  if (typeof left === "number" && typeof right === "number") {
    return (left - right) * factor;
  }

  return String(left).localeCompare(String(right), undefined, {
    sensitivity: "base",
    numeric: true,
  }) * factor;
}

export function columnSortValue(
  row: LibraryTableRow,
  column: LibraryTableColumn,
): string | number | null {
  if (row.source === "library") {
    const track = row.track;
    switch (column) {
      case "title":
        return rowTitle(row);
      case "artist":
        return track.artist;
      case "album":
        return track.album;
      case "genre":
        return track.genre;
      case "bpm":
        return track.bpm;
      case "key":
        return track.key;
      case "duration":
        return track.duration_secs;
      case "path":
        return track.path;
      default: {
        const exhaustive: never = column;
        return exhaustive;
      }
    }
  }

  const known = filesystemMetadata(row);
  switch (column) {
    case "title":
      return rowTitle(row);
    case "artist":
      return known?.artist ?? null;
    case "album":
      return known?.album ?? null;
    case "genre":
      return known?.genre ?? null;
    case "bpm":
      return known?.bpm ?? null;
    case "key":
      return known?.key ?? null;
    case "duration":
      return known?.duration_secs ?? null;
    case "path":
      return row.file.path;
    default: {
      const exhaustive: never = column;
      return exhaustive;
    }
  }
}

export function formatColumnValue(
  row: LibraryTableRow,
  column: LibraryTableColumn,
): string {
  if (row.source === "library") {
    const track = row.track;
    switch (column) {
      case "title":
        return rowTitle(row);
      case "artist":
        return formatOptional(track.artist);
      case "album":
        return formatOptional(track.album);
      case "genre":
        return formatOptional(track.genre);
      case "bpm":
        return formatBpm(track.bpm);
      case "key":
        return formatOptional(track.key);
      case "duration":
        return formatDuration(track.duration_secs);
      case "path":
        return track.path;
      default: {
        const exhaustive: never = column;
        return exhaustive;
      }
    }
  }

  const known = filesystemMetadata(row);
  switch (column) {
    case "title":
      return rowTitle(row);
    case "artist":
      return formatOptional(known?.artist);
    case "album":
      return formatOptional(known?.album);
    case "genre":
      return formatOptional(known?.genre);
    case "bpm":
      return formatBpm(known?.bpm);
    case "key":
      return formatOptional(known?.key);
    case "duration":
      return formatDuration(known?.duration_secs);
    case "path":
      return row.file.path;
    default: {
      const exhaustive: never = column;
      return exhaustive;
    }
  }
}

export function rowMatchesFilter(row: LibraryTableRow, filter: string): boolean {
  const query = filter.trim().toLowerCase();
  if (!query) {
    return true;
  }

  const haystack = [
    rowTitle(row),
    rowPath(row),
    row.source === "library"
      ? row.track.artist
      : row.libraryTrack?.artist ?? null,
    row.source === "library"
      ? row.track.album
      : row.libraryTrack?.album ?? null,
    row.source === "library"
      ? row.track.genre
      : row.libraryTrack?.genre ?? null,
    row.source === "library"
      ? row.track.key
      : row.libraryTrack?.key ?? null,
    row.source === "library"
      ? formatBpm(row.track.bpm)
      : row.libraryTrack
        ? formatBpm(row.libraryTrack.bpm)
        : null,
  ]
    .filter((value): value is string => Boolean(value))
    .join(" ")
    .toLowerCase();

  return haystack.includes(query);
}

export function sortLibraryRows(
  rows: LibraryTableRow[],
  column: LibraryTableColumn,
  direction: SortDirection,
): LibraryTableRow[] {
  return [...rows].sort((left, right) =>
    compareValues(
      columnSortValue(left, column),
      columnSortValue(right, column),
      direction,
    ),
  );
}

export function normalizeLibraryTableColumns(
  columns: LibraryTableColumn[],
): LibraryTableColumn[] {
  const allowed = new Set(LIBRARY_TABLE_COLUMNS.map((column) => column.id));
  const normalized = columns.filter((column) => allowed.has(column));
  if (!normalized.includes("title")) {
    normalized.unshift("title");
  }
  return normalized;
}
