export const DECK_LABELS = ["Deck A", "Deck B"] as const;

export const buttonBase =
  "rounded-lg border px-4 py-2.5 text-sm font-medium transition disabled:cursor-not-allowed disabled:opacity-45";

export const buttonCompact =
  "rounded-md border px-2.5 py-1 text-xs font-medium transition disabled:cursor-not-allowed disabled:opacity-45";

export function statusPillClass(active: boolean): string {
  return active
    ? "rounded-full bg-emerald-500/15 px-2.5 py-0.5 text-xs font-semibold uppercase tracking-wide text-emerald-300"
    : "rounded-full bg-white/8 px-2.5 py-0.5 text-xs font-semibold uppercase tracking-wide text-slate-400";
}
