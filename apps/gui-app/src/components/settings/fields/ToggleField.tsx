import { SettingsToggle } from "@/components/settings/SettingsToggle";
import { useFieldContext } from "@/components/settings/form-context";

interface ToggleFieldProps {
  label: string;
}

export function ToggleField({ label }: ToggleFieldProps) {
  const field = useFieldContext<boolean>();

  return (
    <SettingsToggle
      label={label}
      checked={field.state.value}
      onCheckedChange={(checked) => field.handleChange(checked)}
    />
  );
}
