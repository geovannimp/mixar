import { Input } from "@/components/ui/input";
import { SettingsField } from "@/components/settings/SettingsField";
import { useFieldContext } from "@/components/settings/form-context";

interface NumberFieldProps {
  label: string;
  "aria-label"?: string;
  min?: number;
  max?: number;
  step?: number;
  hint?: string;
}

export function NumberField({
  label,
  "aria-label": ariaLabel,
  min,
  max,
  step,
  hint,
}: NumberFieldProps) {
  const field = useFieldContext<number>();

  return (
    <SettingsField label={label} hint={hint}>
      <Input
        type="number"
        aria-label={ariaLabel ?? label}
        min={min}
        max={max}
        step={step}
        value={field.state.value}
        onBlur={field.handleBlur}
        onChange={(event) => {
          const next = Number(event.target.value);
          field.handleChange(Number.isFinite(next) ? next : field.state.value);
        }}
      />
    </SettingsField>
  );
}
