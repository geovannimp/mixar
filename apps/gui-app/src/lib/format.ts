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

/** Format media duration from milliseconds as `m:ss`. */
export function formatDuration(ms: number | null | undefined): string {
  if (ms == null || !Number.isFinite(ms) || ms < 0) return "—";
  const total = Math.floor(ms / 1000);
  const minutes = Math.floor(total / 60);
  const seconds = total % 60;
  return `${minutes}:${seconds.toString().padStart(2, "0")}`;
}

/** mm:ss.t with one decimal second (DJ-style); input is milliseconds. */
export function formatDeckTimeTenth(ms: number | null | undefined): string {
  if (ms == null || !Number.isFinite(ms) || ms < 0) return "—";
  const secs = Math.max(0, ms) / 1000;
  const minutes = Math.floor(secs / 60);
  const seconds = secs % 60;
  const whole = Math.floor(seconds);
  const tenth = Math.floor((seconds - whole) * 10);
  return `${minutes}:${whole.toString().padStart(2, "0")}.${tenth}`;
}

export function formatDeckRemainingDisplay(
  positionMs: number | null | undefined,
  durationMs: number | null | undefined,
): string {
  if (
    positionMs == null ||
    durationMs == null ||
    !Number.isFinite(positionMs) ||
    !Number.isFinite(durationMs)
  ) {
    return "—";
  }
  const remaining = Math.max(0, durationMs - positionMs);
  return `-${formatDeckTimeTenth(remaining)}`;
}

export function formatDeckTotalDisplay(durationMs: number | null | undefined): string {
  return formatDeckTimeTenth(durationMs);
}

export function effectiveBpm(bpm: number | null | undefined, speed: number): number | null {
  if (bpm == null || !Number.isFinite(bpm)) return null;
  return bpm * speed;
}

/** Default DJ time signature: 4/4. */
export const DEFAULT_BEATS_PER_BAR = 4;

/** Bars per jog position cycle. */
export const JOG_BAR_CYCLE_LENGTH = 4;

/** Playhead progress within a repeating bar window (0–1 per cycle). */
export function barCycleProgress(
  positionMs: number,
  bpm: number,
  beatsPerBar: number = DEFAULT_BEATS_PER_BAR,
  cycleBars: number = JOG_BAR_CYCLE_LENGTH,
): number {
  if (!Number.isFinite(positionMs)) {
    return 0;
  }
  const cycleDurationMs = barCycleDurationMs(bpm, beatsPerBar, cycleBars);
  if (cycleDurationMs == null) {
    return 0;
  }
  const positionInCycle = ((positionMs % cycleDurationMs) + cycleDurationMs) % cycleDurationMs;
  return positionInCycle / cycleDurationMs;
}

/** Continuous jog tracker angle (no 0–360 wrap) to avoid transition glitches at cycle boundaries. */
export function barCycleRotationDeg(
  positionMs: number,
  bpm: number,
  beatsPerBar: number = DEFAULT_BEATS_PER_BAR,
  cycleBars: number = JOG_BAR_CYCLE_LENGTH,
): number {
  if (!Number.isFinite(positionMs)) {
    return 0;
  }
  const cycleDurationMs = barCycleDurationMs(bpm, beatsPerBar, cycleBars);
  if (cycleDurationMs == null) {
    return 0;
  }
  return (positionMs / cycleDurationMs) * 360;
}

function barCycleDurationMs(bpm: number, beatsPerBar: number, cycleBars: number): number | null {
  if (!Number.isFinite(bpm) || bpm <= 0) {
    return null;
  }
  const beatsInCycle = cycleBars * beatsPerBar;
  const cycleDurationMs = ((beatsInCycle * 60) / bpm) * 1000;
  if (cycleDurationMs <= 0) {
    return null;
  }
  return cycleDurationMs;
}

export function getBarCycleDurationMs(bpm: number): number | null {
  return barCycleDurationMs(bpm, DEFAULT_BEATS_PER_BAR, JOG_BAR_CYCLE_LENGTH);
}

/** @deprecated Prefer {@link getBarCycleDurationMs}. */
export function getBarCycleDurationSecs(bpm: number): number | null {
  const ms = getBarCycleDurationMs(bpm);
  return ms == null ? null : ms / 1000;
}

export const PITCH_RANGE_PERCENT = 8;

function clampSpeedToPitchRange(speed: number): number {
  const min = 1 - PITCH_RANGE_PERCENT / 100;
  const max = 1 + PITCH_RANGE_PERCENT / 100;
  return Math.min(max, Math.max(min, speed));
}

/** Map speed to slider 0–100 (center = 50, ±8%). */
export function speedToPitchSlider(speed: number): number {
  const pitch = (clampSpeedToPitchRange(speed) - 1) * 100;
  const clamped = Math.min(PITCH_RANGE_PERCENT, Math.max(-PITCH_RANGE_PERCENT, pitch));
  const raw = ((clamped + PITCH_RANGE_PERCENT) / (2 * PITCH_RANGE_PERCENT)) * 100;
  // Keep sub-step precision so the fader does not quantize to 0.16% pitch jumps.
  return Math.round(raw * 100) / 100;
}

export function pitchSliderToSpeed(value: number): number {
  const clamped = Math.min(100, Math.max(0, value));
  const pitch = (clamped / 100) * (2 * PITCH_RANGE_PERCENT) - PITCH_RANGE_PERCENT;
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
export function formatDeckElapsed(ms: number | null | undefined): string {
  return formatDeckTimeTenth(ms);
}

/** @deprecated use formatDeckRemainingDisplay */
export function formatDeckRemaining(
  positionMs: number | null | undefined,
  durationMs: number | null | undefined,
): string {
  return formatDeckRemainingDisplay(positionMs, durationMs);
}

export function deckDisplayTitle(deck: { title: string | null; track: string | null }): string {
  if (deck.title?.trim()) {
    return deck.title.trim();
  }
  return fileName(deck.track);
}
