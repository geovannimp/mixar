import type { AudioDeviceSummary } from "@/types";
import { SettingsField } from "./SettingsField";
import { SettingsSelect, type SettingsSelectOption } from "./SettingsSelect";

interface DeviceSelectProps {
  label: string;
  hint?: string;
  value: string;
  devices: AudioDeviceSummary[];
  loading?: boolean;
  onChange: (deviceId: string) => void;
}

function deviceOptions(
  devices: AudioDeviceSummary[],
  value: string,
  loading: boolean,
): SettingsSelectOption[] {
  if (devices.length === 0) {
    return [{ value, label: loading ? "Loading devices…" : value }];
  }

  const options = devices.map((device) => ({
    value: device.id,
    label: `${device.name}${device.is_default ? " (default)" : ""}`,
  }));

  if (devices.some((device) => device.id === value) || value === "") {
    return options;
  }

  return [...options, { value, label: value }];
}

export function DeviceSelect({
  label,
  hint,
  value,
  devices,
  loading = false,
  onChange,
}: DeviceSelectProps) {
  const options = deviceOptions(devices, value, loading);

  return (
    <SettingsField label={label} hint={hint}>
      <SettingsSelect
        aria-label={label}
        value={value}
        options={options}
        disabled={loading}
        onValueChange={onChange}
      />
    </SettingsField>
  );
}
