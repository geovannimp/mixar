import { Button } from "@/components/ui/button";
import type { PathBreadcrumb } from "../lib/driveVolumes";

interface DrivePathBreadcrumbsProps {
  crumbs: PathBreadcrumb[];
  listingPath: string;
  busy: boolean;
  embedded?: boolean;
  onOpenDirectory: (path: string) => void;
}

export function DrivePathBreadcrumbs({
  crumbs,
  listingPath,
  busy,
  embedded = false,
  onOpenDirectory,
}: DrivePathBreadcrumbsProps) {
  if (crumbs.length === 0) {
    return null;
  }

  return (
    <nav
      aria-label="Folder path"
      className={embedded ? "inline-flex min-w-0 items-center" : "min-w-0 flex-1"}
      title={listingPath}
    >
      <ol className="flex min-w-0 flex-wrap items-center gap-0.5">
        {crumbs.map((crumb, index) => (
          <li key={crumb.path} className="flex min-w-0 items-center gap-0.5">
            {index > 0 && (
              <span
                className={
                  embedded
                    ? "text-sm text-zinc-600"
                    : "px-0.5 text-xs text-zinc-600"
                }
                aria-hidden
              >
                /
              </span>
            )}
            {embedded ? (
              <button
                type="button"
                className="max-w-40 truncate text-sm leading-5 text-zinc-500 hover:text-zinc-300 disabled:cursor-not-allowed disabled:opacity-50"
                disabled={busy}
                title={crumb.path}
                onClick={() => onOpenDirectory(crumb.path)}
              >
                {crumb.label}
              </button>
            ) : (
              <Button
                type="button"
                variant="ghost"
                size="xs"
                className="max-w-40 truncate px-1.5 text-zinc-400"
                disabled={busy}
                title={crumb.path}
                onClick={() => onOpenDirectory(crumb.path)}
              >
                {crumb.label}
              </Button>
            )}
          </li>
        ))}
      </ol>
    </nav>
  );
}
