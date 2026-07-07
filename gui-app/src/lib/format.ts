export function fileName(path: string | null): string {
  if (!path) return "No track loaded";
  const parts = path.split(/[/\\]/);
  return parts[parts.length - 1] ?? path;
}
