export const DECK_LABELS = ["Deck A", "Deck B"] as const;

export type DeckAccent = "a" | "b";

export const DECK_ACCENTS: Record<DeckAccent, { label: string; ring: string; border: string; bg: string; text: string; button: string; waveform: string }> = {
  a: {
    label: "Deck A",
    ring: "border-sky-400/60",
    border: "border-sky-500/30",
    bg: "bg-sky-500/5",
    text: "text-sky-300",
    button: "border-sky-500/45 bg-sky-500/15 hover:bg-sky-500/25",
    waveform: "from-sky-500/40 via-sky-400/20 to-sky-500/5",
  },
  b: {
    label: "Deck B",
    ring: "border-rose-400/60",
    border: "border-rose-500/30",
    bg: "bg-rose-500/5",
    text: "text-rose-300",
    button: "border-rose-500/45 bg-rose-500/15 hover:bg-rose-500/25",
    waveform: "from-rose-500/40 via-rose-400/20 to-rose-500/5",
  },
};

export const buttonBase =
  "rounded border px-4 py-2.5 text-sm font-medium transition disabled:cursor-not-allowed disabled:opacity-45";

export const buttonCompact =
  "rounded border px-2.5 py-1 text-xs font-medium transition disabled:cursor-not-allowed disabled:opacity-45";

export const buttonIcon =
  "flex h-6 w-6 items-center justify-center rounded border text-sm font-semibold leading-none transition disabled:cursor-not-allowed disabled:opacity-45";

export const buttonTransport =
  "rounded border px-5 py-2 text-sm font-semibold uppercase tracking-wide transition disabled:cursor-not-allowed disabled:opacity-45";

export function statusPillClass(active: boolean): string {
  return active
    ? "rounded-full bg-emerald-500/15 px-2.5 py-0.5 text-xs font-semibold uppercase tracking-wide text-emerald-300"
    : "rounded-full bg-white/8 px-2.5 py-0.5 text-xs font-semibold uppercase tracking-wide text-slate-400";
}
