import {
  Select,
  SelectItem,
  SelectPopup,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { withForm } from "@/components/settings/form";
import { SettingsField, SettingsSectionHeader } from "@/components/settings/SettingsField";
import { settingsFormOptions } from "@/components/settings/settingsFormOptions";
import {
  ANALYSIS_MODE_OPTIONS,
  findAnalysisModeOption,
  type AnalysisModeOption,
} from "@/lib/analysisMode";
import { LIBRARY_TABLE_COLUMNS, normalizeLibraryTableColumns } from "@/lib/libraryTable";
import type { LibraryTableColumn } from "@/types";

function AnalysisModeOptionLabel({ item }: { item: AnalysisModeOption }) {
  return (
    <span className="flex flex-col text-left">
      <span className="truncate">{item.label}</span>
      <span className="truncate text-muted-foreground text-xs">{item.description}</span>
    </span>
  );
}

export const SettingsLibraryPanel = withForm({
  ...settingsFormOptions,
  render: function Render({ form }) {
    return (
      <div className="space-y-5">
        <SettingsSectionHeader
          title="Library"
          description="Track import and offline analysis behavior."
        />

        <form.AppField name="analysis_duration">
          {(field) => {
            const selected = findAnalysisModeOption(field.state.value);
            return (
              <SettingsField label="Analysis quality">
                <Select
                  aria-label="Analysis quality"
                  value={selected}
                  onValueChange={(item) => {
                    if (!item) {
                      return;
                    }
                    field.handleChange(item.value);
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
            );
          }}
        </form.AppField>

        <form.AppField name="scan_folder_tree">
          {(field) => <field.ToggleField label="Scan folder collections recursively" />}
        </form.AppField>

        <SettingsField label="Track table columns">
          <form.Subscribe selector={(state) => state.values.library_table_columns}>
            {(columns) => (
              <div className="space-y-2 rounded-lg border border-white/10 bg-black/20 p-3">
                {LIBRARY_TABLE_COLUMNS.map((column) => {
                  const checked = column.required || columns.includes(column.id);
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
                          const next = new Set(columns);
                          if (event.target.checked) {
                            next.add(column.id);
                          } else {
                            next.delete(column.id);
                          }
                          form.setFieldValue(
                            "library_table_columns",
                            normalizeLibraryTableColumns(Array.from(next) as LibraryTableColumn[]),
                          );
                        }}
                      />
                    </label>
                  );
                })}
              </div>
            )}
          </form.Subscribe>
        </SettingsField>
      </div>
    );
  },
});
