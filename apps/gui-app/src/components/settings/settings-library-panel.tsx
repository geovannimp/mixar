import {
  Select,
  SelectItem,
  SelectPopup,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  ANALYSIS_MODE_OPTIONS,
  findAnalysisModeOption,
  type AnalysisModeOption,
} from "@/lib/analysis-mode";
import type { AppSettings, LibraryTableColumn } from "@/types";
import { SettingsField, SettingsSectionHeader } from "./settings-field";
import { SettingsToggle } from "./settings-toggle";
import { LIBRARY_TABLE_COLUMNS, normalizeLibraryTableColumns } from "@/lib/library-table";

interface SettingsLibraryPanelProps {
  draft: AppSettings;
  onChange: (next: AppSettings) => void;
}

function AnalysisModeOptionLabel({ item }: { item: AnalysisModeOption }) {
  return (
    <span className="flex flex-col text-left">
      <span className="truncate">{item.label}</span>
      <span className="truncate text-muted-foreground text-xs">{item.description}</span>
    </span>
  );
}

export function SettingsLibraryPanel({ draft, onChange }: SettingsLibraryPanelProps) {
  const selected = findAnalysisModeOption(draft.analysis_duration);

  return (
    <div className="space-y-5">
      <SettingsSectionHeader
        title="Library"
        description="Track import and offline analysis behavior."
      />

      <SettingsField label="Analysis quality">
        <Select
          aria-label="Analysis quality"
          value={selected}
          onValueChange={(item) => {
            if (!item) {
              return;
            }
            onChange({
              ...draft,
              analysis_duration: item.value,
            });
          }}
          itemToStringValue={(item) => item.value}
        >
          <SelectTrigger className="h-auto py-1">
            <SelectValue>{(item) => <AnalysisModeOptionLabel item={item} />}</SelectValue>
          </SelectTrigger>
          <SelectPopup>
            {ANALYSIS_MODE_OPTIONS.map((item) => (
              <SelectItem key={item.value} value={item}>
                <AnalysisModeOptionLabel item={item} />
              </SelectItem>
            ))}
          </SelectPopup>
        </Select>
      </SettingsField>

      <SettingsToggle
        label="Scan folder collections recursively"
        checked={draft.scan_folder_tree}
        onCheckedChange={(scan_folder_tree) => onChange({ ...draft, scan_folder_tree })}
      />

      <SettingsField label="Track table columns">
        <div className="space-y-2 rounded-lg border border-white/10 bg-black/20 p-3">
          {LIBRARY_TABLE_COLUMNS.map((column) => {
            const checked = column.required || draft.library_table_columns.includes(column.id);
            return (
              <label
                key={column.id}
                className="flex items-center justify-between gap-3 text-sm text-zinc-300"
              >
                <span>{column.label}</span>
                <input
                  type="checkbox"
                  className="size-4 rounded border-white/20 bg-zinc-900 accent-emerald-500"
                  checked={checked}
                  disabled={column.required}
                  onChange={(event) => {
                    const next = new Set(draft.library_table_columns);
                    if (event.target.checked) {
                      next.add(column.id);
                    } else {
                      next.delete(column.id);
                    }
                    onChange({
                      ...draft,
                      library_table_columns: normalizeLibraryTableColumns(
                        Array.from(next) as LibraryTableColumn[],
                      ),
                    });
                  }}
                />
              </label>
            );
          })}
        </div>
      </SettingsField>
    </div>
  );
}
