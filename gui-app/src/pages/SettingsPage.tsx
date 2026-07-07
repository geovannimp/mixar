import { useEffect, useState } from "react";
import { MessageBanner } from "../components/MessageBanner";
import { SettingsAudioPanel } from "../components/settings/SettingsAudioPanel";
import { SettingsLibraryPanel } from "../components/settings/SettingsLibraryPanel";
import { SettingsSidebar } from "../components/settings/SettingsSidebar";
import { useAudioDevices } from "../hooks/useAudioDevices";
import { useSettings } from "../hooks/useSettings";
import { normalizeAppSettings } from "../lib/busSettings";
import { buttonBase } from "../lib/ui";
import type { AppSettings, AudioDeviceSummary, SettingsSection } from "../types";

function SettingsSectionPanel({
  section,
  draft,
  devices,
  devicesLoading,
  onChange,
}: {
  section: SettingsSection;
  draft: AppSettings;
  devices: AudioDeviceSummary[];
  devicesLoading: boolean;
  onChange: (next: AppSettings) => void;
}) {
  switch (section) {
    case "audio":
      return (
        <SettingsAudioPanel
          draft={draft}
          devices={devices}
          devicesLoading={devicesLoading}
          onChange={onChange}
        />
      );
    case "library":
      return <SettingsLibraryPanel draft={draft} onChange={onChange} />;
    default: {
      const exhaustive: never = section;
      return exhaustive;
    }
  }
}

export function SettingsPage() {
  const { settings, error, busy, saved, save } = useSettings();
  const [draft, setDraft] = useState<AppSettings | null>(null);
  const [section, setSection] = useState<SettingsSection>("audio");
  const { devices, loading: devicesLoading } = useAudioDevices(
    draft?.backend ?? settings?.backend ?? "cpal",
  );

  useEffect(() => {
    if (settings) {
      setDraft(normalizeAppSettings(settings));
    }
  }, [settings]);

  if (!draft) {
    return (
      <div className="flex flex-1 items-center justify-center text-sm text-zinc-500">
        Loading settings…
      </div>
    );
  }

  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
      <div className="shrink-0 border-b border-white/8 px-4 py-3 sm:px-6">
        <h1 className="text-xs font-bold uppercase tracking-widest text-zinc-400">
          Settings
        </h1>
        <p className="mt-1 text-sm text-zinc-500">
          Stop the engine before saving changes.
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

        <form
          className="flex min-h-0 flex-1 flex-col"
          onSubmit={(event) => {
            event.preventDefault();
            void save(draft);
          }}
        >
          <div className="min-h-0 flex-1 overflow-y-auto p-4 sm:p-6">
            <div className="max-w-2xl">
              <SettingsSectionPanel
                section={section}
                draft={draft}
                devices={devices}
                devicesLoading={devicesLoading}
                onChange={setDraft}
              />
            </div>
          </div>

          <div className="shrink-0 border-t border-white/8 px-4 py-3 sm:px-6">
            <button
              type="submit"
              className={`${buttonBase} border-emerald-500/45 bg-emerald-500/15 hover:bg-emerald-500/25`}
              disabled={busy}
            >
              {busy ? "Saving…" : "Save"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
