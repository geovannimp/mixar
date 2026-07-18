import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";

interface SettingsToggleProps {
  label: string;
  checked: boolean;
  onCheckedChange: (checked: boolean) => void;
}

export function SettingsToggle({ label, checked, onCheckedChange }: SettingsToggleProps) {
  return (
    <Label className="cursor-pointer font-normal text-muted-foreground">
      <Switch checked={checked} onCheckedChange={onCheckedChange} />
      {label}
    </Label>
  );
}
