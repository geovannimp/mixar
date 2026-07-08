import { Folder } from "lucide-react";
import type { ReactNode } from "react";
import { TrackActionsMenu } from "./TrackActionsMenu";

interface DriveFolderRowProps {
  busy: boolean;
  title: string;
  indented?: boolean;
  selected?: boolean;
  label: ReactNode;
  onOpen?: () => void;
  onCreateCollection: () => void;
}

export function DriveFolderRow({
  busy,
  title,
  indented = false,
  selected = false,
  label,
  onOpen,
  onCreateCollection,
}: DriveFolderRowProps) {
  const rowClass = selected
    ? "border-l-emerald-500/60 bg-white/5"
    : "border-l-transparent hover:bg-white/5";

  const folderIcon = (
    <Folder
      className={`size-4 shrink-0 ${selected ? "text-zinc-400" : "text-zinc-500"}`}
      aria-hidden
    />
  );

  const mainContent = onOpen ? (
    <button
      type="button"
      className="flex min-w-0 flex-1 items-center gap-2 text-left"
      disabled={busy}
      title={title}
      onClick={onOpen}
    >
      {folderIcon}
      <div className="min-w-0 flex-1">{label}</div>
    </button>
  ) : (
    <div
      className="flex min-w-0 flex-1 items-center gap-2"
      aria-current="location"
      title={title}
    >
      {folderIcon}
      <div className="min-w-0 flex-1">{label}</div>
    </div>
  );

  return (
    <div
      className={`group flex items-center gap-1 rounded border-l-2 py-2 pr-1 ${indented ? "pl-6" : "pl-3"} ${rowClass}`}
    >
      {mainContent}
      <TrackActionsMenu
        busy={busy}
        hiddenUntilHover
        menuLabel="Folder actions"
        actions={[
          {
            label: "Create collection",
            onClick: onCreateCollection,
          },
        ]}
      />
    </div>
  );
}
