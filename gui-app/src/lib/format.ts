export function fileName(path: string | null): string {
  if (!path) return "No track loaded";
  const parts = path.split(/[/\\]/);
  return parts[parts.length - 1] ?? path;
}

export function formatOptional(value: string | null | undefined): string {
  return value?.trim() ? value : "—";
}

export function formatBpm(bpm: number | null | undefined): string {
  if (bpm == null || !Number.isFinite(bpm)) return "—";
  return bpm.toFixed(2);
}

export function formatDuration(secs: number | null | undefined): string {
  if (secs == null || !Number.isFinite(secs) || secs < 0) return "—";
  const total = Math.floor(secs);
  const minutes = Math.floor(total / 60);
  const seconds = total % 60;
  return `${minutes}:${seconds.toString().padStart(2, "0")}`;
}

/** mm:ss.t with one decimal second (DJ-style). */
export function formatDeckTimeTenth(secs: number | null | undefined): string {
  if (secs == null || !Number.isFinite(secs) || secs < 0) return "—";
  const clamped = Math.max(0, secs);
  const minutes = Math.floor(clamped / 60);
  const seconds = clamped % 60;
  const whole = Math.floor(seconds);
  const tenth = Math.floor((seconds - whole) * 10);
  return `${minutes}:${whole.toString().padStart(2, "0")}.${tenth}`;
}

export function formatDeckRemainingDisplay(
  positionSecs: number | null | undefined,
  durationSecs: number | null | undefined,
): string {
  if (
    positionSecs == null ||
    durationSecs == null ||
    !Number.isFinite(positionSecs) ||
    !Number.isFinite(durationSecs)
  ) {
    return "—";
  }
  const remaining = Math.max(0, durationSecs - positionSecs);
  return `-${formatDeckTimeTenth(remaining)}`;
}

export function formatDeckTotalDisplay(
  durationSecs: number | null | undefined,
): string {
  return formatDeckTimeTenth(durationSecs);
}

export function effectiveBpm(
  bpm: number | null | undefined,
  speed: number,
): number | null {
  if (bpm == null || !Number.isFinite(bpm)) return null;
  return bpm * speed;
}

export const PITCH_RANGE_PERCENT = 8;

function clampSpeedToPitchRange(speed: number): number {
  const min = 1 - PITCH_RANGE_PERCENT / 100;
  const max = 1 + PITCH_RANGE_PERCENT / 100;
  return Math.min(max, Math.max(min, speed));
}

/** Map speed to slider 0–100 (center = 50, ±8%). */
export function speedToPitchSlider(speed: number): number {
  const pitch = ((clampSpeedToPitchRange(speed) - 1) * 100);
  const clamped = Math.min(PITCH_RANGE_PERCENT, Math.max(-PITCH_RANGE_PERCENT, pitch));
  return Math.round(((clamped + PITCH_RANGE_PERCENT) / (2 * PITCH_RANGE_PERCENT)) * 100);
}

export function pitchSliderToSpeed(value: number): number {
  const clamped = Math.min(100, Math.max(0, value));
  const pitch =
    (clamped / 100) * (2 * PITCH_RANGE_PERCENT) - PITCH_RANGE_PERCENT;
  return clampSpeedToPitchRange(1 + pitch / 100);
}

export function nudgeSpeed(speed: number, deltaPercent: number): number {
  return clampSpeedToPitchRange(speed + deltaPercent / 100);
}

/** Mixxx-style pitch readout (e.g. 0.00, -1.25). */
export function formatPitchOffset(speed: number): string {
  const percent = (speed - 1) * 100;
  const sign = percent >= 0 ? "+" : "";
  return `${sign}${percent.toFixed(2)}`;
}

export function formatPitchPercent(speed: number): string {
  return `${formatPitchOffset(speed)}%`;
}

/** @deprecated use formatDeckTimeTenth */
export function formatDeckElapsed(secs: number | null | undefined): string {
  return formatDeckTimeTenth(secs);
}

/** @deprecated use formatDeckRemainingDisplay */
export function formatDeckRemaining(
  positionSecs: number | null | undefined,
  durationSecs: number | null | undefined,
): string {
  return formatDeckRemainingDisplay(positionSecs, durationSecs);
}

export function deckDisplayTitle(deck: {
  title: string | null;
  track: string | null;
}): string {
  if (deck.title?.trim()) {
    return deck.title.trim();
  }
  return fileName(deck.track);
}
