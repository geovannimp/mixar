import { SettingsField } from "@/components/settings/SettingsField";
import { SettingsSelect, type SettingsSelectOption } from "@/components/settings/SettingsSelect";
import { useFieldContext } from "@/components/settings/form-context";

interface SelectFieldProps<T extends string> {
  label: string;
  "aria-label"?: string;
  options: SettingsSelectOption<T>[];
  disabled?: boolean;
  hint?: string;
}

export function SelectField<T extends string>({
  label,
  "aria-label": ariaLabel,
  options,
  disabled,
  hint,
}: SelectFieldProps<T>) {
  const field = useFieldContext<T>();

  return (
    <SettingsField label={label} hint={hint}>
      <SettingsSelect
        aria-label={ariaLabel ?? label}
        value={field.state.value}
        options={options}
        disabled={disabled}
        onValueChange={(value) => field.handleChange(value)}
      />
    </SettingsField>
  );
}
