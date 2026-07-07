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
} from "@/lib/analysisMode";
import type { AppSettings } from "@/types";
import { SettingsField, SettingsSectionHeader } from "./SettingsField";
import { SettingsToggle } from "./SettingsToggle";

interface SettingsLibraryPanelProps {
  draft: AppSettings;
  onChange: (next: AppSettings) => void;
}

function AnalysisModeOptionLabel({ item }: { item: AnalysisModeOption }) {
  return (
    <span className="flex flex-col text-left">
      <span className="truncate">{item.label}</span>
      <span className="truncate text-muted-foreground text-xs">
        {item.description}
      </span>
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
            <SelectValue>
              {(item) => <AnalysisModeOptionLabel item={item} />}
            </SelectValue>
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
        onCheckedChange={(scan_folder_tree) =>
          onChange({ ...draft, scan_folder_tree })
        }
      />
    </div>
  );
}
