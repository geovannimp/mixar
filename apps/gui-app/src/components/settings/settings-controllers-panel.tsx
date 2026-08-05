import { useCallback, useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { toastManager } from "@/components/ui/toast";
import {
  getControllerTransport,
  type ControllerDeviceInfo,
  type ControllerMappingInfo,
} from "@/lib/controller/transport";
import { SettingsField, SettingsSectionHeader } from "./settings-field";

export function SettingsControllersPanel() {
  const [mappings, setMappings] = useState<ControllerMappingInfo[]>([]);
  const [devices, setDevices] = useState<ControllerDeviceInfo[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setError(null);
    try {
      const transport = getControllerTransport();
      const [nextMappings, nextDevices] = await Promise.all([
        transport.listMappings(),
        transport.listDevices(),
      ]);
      setMappings(nextMappings);
      setDevices(nextDevices);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  async function run(action: () => Promise<void>) {
    setBusy(true);
    setError(null);
    try {
      await action();
      await refresh();
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
      toastManager.add({ title: message, type: "error" });
    } finally {
      setBusy(false);
    }
  }

  const transport = getControllerTransport();

  return (
    <div className="space-y-5">
      <SettingsSectionHeader
        title="Controllers"
        description="MIDI mappings live in app data. Seed copies shipped maps when missing; Update overwrites from the app bundle."
      />

      {error && <p className="text-sm text-red-400">{error}</p>}

      <div className="flex flex-wrap gap-2">
        <Button
          type="button"
          variant="outline"
          disabled={busy}
          onClick={() => void run(() => transport.updateAllMappings())}
        >
          Update all mappings
        </Button>
        <Button type="button" variant="outline" disabled={busy} onClick={() => void refresh()}>
          Refresh
        </Button>
      </div>

      <SettingsField label="Mappings">
        <div className="space-y-2 rounded-lg border border-white/10 bg-black/20 p-3">
          {mappings.length === 0 && (
            <p className="text-sm text-zinc-500">No mappings in app data yet.</p>
          )}
          {mappings.map((mapping) => (
            <div
              key={mapping.id}
              className="flex flex-col gap-2 border-b border-white/5 py-2 last:border-b-0 sm:flex-row sm:items-center sm:justify-between"
            >
              <div className="min-w-0">
                <p className="truncate text-sm font-medium text-zinc-100">
                  {mapping.name}
                  {mapping.attached ? (
                    <span className="ml-2 text-xs font-normal text-emerald-400">attached</span>
                  ) : null}
                </p>
                <p className="truncate text-xs text-zinc-500">
                  {mapping.id} · {mapping.device_id}
                </p>
              </div>
              <div className="flex shrink-0 flex-wrap gap-2">
                {mapping.attached ? (
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    disabled={busy}
                    onClick={() => void run(() => transport.disableMapping(mapping.id))}
                  >
                    Disable
                  </Button>
                ) : (
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    disabled={busy}
                    onClick={() => void run(() => transport.enableMapping(mapping.id))}
                  >
                    Enable
                  </Button>
                )}
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  disabled={busy}
                  onClick={() => void run(() => transport.updateMapping(mapping.id))}
                >
                  Update
                </Button>
              </div>
            </div>
          ))}
        </div>
      </SettingsField>

      <SettingsField label="MIDI ports">
        <div className="space-y-1 rounded-lg border border-white/10 bg-black/20 p-3">
          {devices.length === 0 && <p className="text-sm text-zinc-500">No MIDI ports detected.</p>}
          {devices.map((device) => (
            <p
              key={`${device.direction}:${device.port_name}`}
              className="truncate text-xs text-zinc-400"
            >
              <span className="text-zinc-500">{device.direction}</span> {device.port_name}
              {device.matched_mapping_id ? (
                <span className="text-emerald-500/80"> → {device.matched_mapping_id}</span>
              ) : null}
            </p>
          ))}
        </div>
      </SettingsField>
    </div>
  );
}
