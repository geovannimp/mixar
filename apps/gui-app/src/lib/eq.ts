export const EQ_MIN_DB = -24;
export const EQ_MAX_DB = 24;
export const EQ_STEP_DB = 0.1;

/** Absolute strip controls on the wire (`eq` / `filter` / `gain_trim`) are `0..1`. */
export const CONTROL_NORM_MIN = 0;
export const CONTROL_NORM_MAX = 1;
export const CONTROL_NORM_CENTER = 0.5;
/** ~0.1 dB steps across ±24 dB. */
export const CONTROL_NORM_STEP = EQ_STEP_DB / (EQ_MAX_DB - EQ_MIN_DB);

export function clampEqDb(value: number): number {
  return Math.min(EQ_MAX_DB, Math.max(EQ_MIN_DB, value));
}

export function snapEqDb(value: number): number {
  return clampEqDb(Math.round(value / EQ_STEP_DB) * EQ_STEP_DB);
}

export function stripDbToNorm(db: number): number {
  return Math.min(1, Math.max(0, (clampEqDb(db) - EQ_MIN_DB) / (EQ_MAX_DB - EQ_MIN_DB)));
}

export function normToStripDb(norm: number): number {
  const n = Math.min(1, Math.max(0, norm));
  return EQ_MIN_DB + n * (EQ_MAX_DB - EQ_MIN_DB);
}

export function formatStripNormDb(norm: number): string {
  const db = snapEqDb(normToStripDb(norm));
  const sign = db > 0 ? "+" : "";
  return `${sign}${db.toFixed(1)}`;
}
