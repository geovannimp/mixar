export function fileName(path: string | null): string {
  if (!path) return "No track loaded";
  const parts = path.split(/[/\\]/);
  return parts[parts.length - 1] ?? path;
}

export function formatOptional(value: string | null | undefined): string {
  return value?.trim() ? value : "—";
}

export function formatBpm(bpm: number | null | undefined): string {
  if (bpm == null || !Number.isFinite(bpm)) return "—";
  return bpm.toFixed(2);
}

export function formatDuration(secs: number | null | undefined): string {
  if (secs == null || !Number.isFinite(secs) || secs < 0) return "—";
  const total = Math.floor(secs);
  const minutes = Math.floor(total / 60);
  const seconds = total % 60;
  return `${minutes}:${seconds.toString().padStart(2, "0")}`;
}
