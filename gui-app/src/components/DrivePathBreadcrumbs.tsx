import { Button, buttonVariants } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { buildPathBreadcrumbs } from "../lib/driveVolumes";
import type { VolumeInfo } from "../types";

interface DrivePathBreadcrumbsProps {
  listingPath: string;
  selectedVolume: VolumeInfo | null;
  busy: boolean;
  onOpenDirectory: (path: string) => void;
}

export function DrivePathBreadcrumbs({
  listingPath,
  selectedVolume,
  busy,
  onOpenDirectory,
}: DrivePathBreadcrumbsProps) {
  const crumbs = buildPathBreadcrumbs(listingPath, selectedVolume);

  return (
    <nav
      aria-label="Folder path"
      className="min-w-0 flex-1"
      title={listingPath}
    >
      <ol className="flex min-w-0 flex-wrap items-center gap-0.5">
        {crumbs.map((crumb, index) => (
          <li key={crumb.path} className="flex min-w-0 items-center gap-0.5">
            {index > 0 && (
              <span className="px-0.5 text-xs text-zinc-600" aria-hidden>
                /
              </span>
            )}
            {crumb.isCurrent ? (
              <span
                className={cn(
                  buttonVariants({ variant: "ghost", size: "xs" }),
                  "max-w-[10rem] truncate text-zinc-500",
                )}
              >
                {crumb.label}
              </span>
            ) : (
              <Button
                type="button"
                variant="ghost"
                size="xs"
                className="max-w-[10rem] truncate px-1.5 text-zinc-400"
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
