import type { LibrarySourceTab } from "@/types";

interface LibrarySourceTabsProps {
  activeTab: LibrarySourceTab;
  onTabChange: (tab: LibrarySourceTab) => void;
}

const tabs: { id: LibrarySourceTab; label: string }[] = [
  { id: "collections", label: "Collections" },
  { id: "drive", label: "Drive" },
];

export function LibrarySourceTabs({ activeTab, onTabChange }: LibrarySourceTabsProps) {
  return (
    <div
      className="flex shrink-0 gap-1 rounded-md border border-white/8 bg-black/20 p-0.5"
      role="tablist"
      aria-label="Library source"
    >
      {tabs.map((tab) => {
        const selected = tab.id === activeTab;
        return (
          <button
            key={tab.id}
            type="button"
            role="tab"
            aria-selected={selected}
            className={
              selected
                ? "flex-1 rounded px-2 py-1 text-[10px] font-semibold uppercase tracking-widest text-emerald-300 bg-emerald-500/15"
                : "flex-1 rounded px-2 py-1 text-[10px] font-semibold uppercase tracking-widest text-zinc-500 hover:bg-white/5 hover:text-zinc-300"
            }
            onClick={() => onTabChange(tab.id)}
          >
            {tab.label}
          </button>
        );
      })}
    </div>
  );
}
