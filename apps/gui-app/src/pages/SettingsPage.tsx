import { useEffect, useState } from "react";
import { useStore } from "@tanstack/react-form";
import { MessageBanner } from "@/components/MessageBanner";
import { useAppForm } from "@/components/settings/form";
import { SettingsAudioPanel } from "@/components/settings/SettingsAudioPanel";
import { SettingsLibraryPanel } from "@/components/settings/SettingsLibraryPanel";
import { SettingsSidebar } from "@/components/settings/SettingsSidebar";
import { settingsFormOptions } from "@/components/settings/settingsFormOptions";
import { useAudioDevices } from "@/hooks/useAudioDevices";
import { useSettings } from "@/hooks/useSettings";
import { normalizeAppSettings } from "@/lib/busSettings";
import type { AppSettings, SettingsSection } from "@/types";

function SettingsForm({
  settings,
  error,
  busy,
  saved,
  save,
}: {
  settings: AppSettings;
  error: string | null;
  busy: boolean;
  saved: boolean;
  save: (next: AppSettings) => Promise<void>;
}) {
  const [section, setSection] = useState<SettingsSection>("audio");
  const form = useAppForm({
    ...settingsFormOptions,
    defaultValues: normalizeAppSettings(settings),
    onSubmit: async ({ value }) => {
      await save(value);
    },
  });

  useEffect(() => {
    form.reset(normalizeAppSettings(settings));
  }, [form, settings]);

  const backend = useStore(form.store, (state) => state.values.backend);
  const { devices, loading: devicesLoading } = useAudioDevices(backend);

  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
      <div className="shrink-0 border-b border-white/8 px-4 py-3 sm:px-6">
        <h1 className="text-xs font-bold uppercase tracking-widest text-zinc-400">Settings</h1>
        <p className="mt-1 text-sm text-zinc-500">
          Saving restarts the engine automatically if it&apos;s running.
        </p>
        {(error || saved) && (
          <div className="mt-3 space-y-2">
            {error && <MessageBanner message={error} variant="error" />}
            {saved && <MessageBanner message="Settings saved." variant="success" />}
          </div>
        )}
      </div>

      <div className="grid min-h-0 flex-1 grid-cols-1 sm:grid-cols-[11rem_1fr]">
        <SettingsSidebar active={section} onSelect={setSection} />

        <form.AppForm>
          <form
            className="flex min-h-0 flex-1 flex-col"
            onSubmit={(event) => {
              event.preventDefault();
              void form.handleSubmit();
            }}
          >
            <div className="min-h-0 flex-1 overflow-y-auto p-4 sm:p-6">
              <div className="max-w-2xl">
                {(() => {
                  switch (section) {
                    case "audio":
                      return (
                        <SettingsAudioPanel
                          form={form}
                          devices={devices}
                          devicesLoading={devicesLoading}
                        />
                      );
                    case "library":
                      return <SettingsLibraryPanel form={form} />;
                    default: {
                      const exhaustive: never = section;
                      return exhaustive;
                    }
                  }
                })()}
              </div>
            </div>

            <div className="shrink-0 border-t border-white/8 px-4 py-3 sm:px-6">
              <form.SaveButton busy={busy} />
            </div>
          </form>
        </form.AppForm>
      </div>
    </div>
  );
}

export function SettingsPage() {
  const { settings, error, busy, saved, save } = useSettings();

  if (!settings) {
    return (
      <div className="flex flex-1 items-center justify-center text-sm text-zinc-500">
        Loading settings…
      </div>
    );
  }

  return <SettingsForm settings={settings} error={error} busy={busy} saved={saved} save={save} />;
}
