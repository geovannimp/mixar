import {
  Select,
  SelectItem,
  SelectPopup,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

export type SettingsSelectOption<T extends string = string> = {
  label: string;
  value: T;
};

interface SettingsSelectProps<T extends string> {
  "aria-label": string;
  value: T;
  options: SettingsSelectOption<T>[];
  disabled?: boolean;
  onValueChange: (value: T) => void;
}

export function SettingsSelect<T extends string>({
  "aria-label": ariaLabel,
  value,
  options,
  disabled = false,
  onValueChange,
}: SettingsSelectProps<T>) {
  const selected =
    options.find((option) => option.value === value) ?? options[0];

  if (!selected) {
    return null;
  }

  return (
    <Select
      aria-label={ariaLabel}
      disabled={disabled}
      value={selected}
      onValueChange={(item) => {
        if (!item) {
          return;
        }
        onValueChange(item.value);
      }}
      itemToStringValue={(item) => item.value}
    >
      <SelectTrigger>
        <SelectValue />
      </SelectTrigger>
      <SelectPopup>
        {options.map((option) => (
          <SelectItem key={option.value || "__empty__"} value={option}>
            {option.label}
          </SelectItem>
        ))}
      </SelectPopup>
    </Select>
  );
}
