export const EQ_MIN_DB = -24;
export const EQ_MAX_DB = 24;
export const EQ_STEP_DB = 1;

export function clampEqDb(value: number): number {
  return Math.min(EQ_MAX_DB, Math.max(EQ_MIN_DB, value));
}

export function snapEqDb(value: number): number {
  return clampEqDb(Math.round(value / EQ_STEP_DB) * EQ_STEP_DB);
}
