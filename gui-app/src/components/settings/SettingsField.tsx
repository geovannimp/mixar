import type { ReactNode } from "react";

interface SettingsFieldProps {
  label: string;
  hint?: string;
  children: ReactNode;
}

export function SettingsField({ label, hint, children }: SettingsFieldProps) {
  return (
    <label className="block space-y-1.5">
      <span className="text-xs font-semibold uppercase tracking-wide text-zinc-500">
        {label}
      </span>
      {children}
      {hint && <p className="text-xs text-zinc-600">{hint}</p>}
    </label>
  );
}

interface SettingsSectionHeaderProps {
  title: string;
  description: string;
}

export function SettingsSectionHeader({
  title,
  description,
}: SettingsSectionHeaderProps) {
  return (
    <div>
      <h2 className="text-sm font-semibold text-zinc-100">{title}</h2>
      <p className="mt-1 text-sm text-zinc-500">{description}</p>
    </div>
  );
}
