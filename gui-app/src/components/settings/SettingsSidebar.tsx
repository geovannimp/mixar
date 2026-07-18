import type { SettingsSection } from "@/types";

const SECTIONS: { id: SettingsSection; label: string }[] = [
  { id: "audio", label: "Audio" },
  { id: "library", label: "Library" },
];

interface SettingsSidebarProps {
  active: SettingsSection;
  onSelect: (section: SettingsSection) => void;
}

export function SettingsSidebar({ active, onSelect }: SettingsSidebarProps) {
  return (
    <aside className="w-full shrink-0 border-b border-white/8 sm:w-44 sm:border-b-0 sm:border-r">
      <p className="px-4 py-3 text-[10px] font-semibold uppercase tracking-widest text-zinc-600">
        Sections
      </p>
      <nav className="flex gap-1 overflow-x-auto px-2 pb-3 sm:flex-col sm:overflow-x-visible">
        {SECTIONS.map((section) => {
          const selected = section.id === active;
          return (
            <button
              key={section.id}
              type="button"
              className={
                selected
                  ? "shrink-0 rounded border-l-2 border-l-emerald-400 bg-emerald-500/10 px-3 py-2 text-left text-sm font-medium text-zinc-100"
                  : "shrink-0 rounded border-l-2 border-l-transparent px-3 py-2 text-left text-sm text-zinc-400 hover:bg-white/5 hover:text-zinc-200"
              }
              onClick={() => onSelect(section.id)}
            >
              {section.label}
            </button>
          );
        })}
      </nav>
    </aside>
  );
}
