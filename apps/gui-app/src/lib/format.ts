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

/** Pioneer-style tempo span matching engine `control_norm` (±16%). */
export const SPEED_RATIO_MIN = 0.84;
export const SPEED_RATIO_MAX = 1.16;

/** Tempo fader position `0..1` → playback ratio. */
export function normToSpeedRatio(norm: number): number {
  const n = Math.min(1, Math.max(0, norm));
  return SPEED_RATIO_MAX - n * (SPEED_RATIO_MAX - SPEED_RATIO_MIN);
}

/** Playback ratio → tempo fader position `0..1`. */
export function speedRatioToNorm(speed: number): number {
  const s = Math.min(SPEED_RATIO_MAX, Math.max(SPEED_RATIO_MIN, speed));
  return Math.min(1, Math.max(0, 1 - (s - SPEED_RATIO_MIN) / (SPEED_RATIO_MAX - SPEED_RATIO_MIN)));
}

export function effectiveBpm(bpm: number | null | undefined, speedNorm: number): number | null {
  if (bpm == null || !Number.isFinite(bpm)) return null;
  return bpm * normToSpeedRatio(speedNorm);
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

/** @deprecated Engine pitch span is ±16%; kept for callers that still assume ±8% UI. */
export const PITCH_RANGE_PERCENT = 8;

/** Map tempo fader position `0..1` to slider 0–100. */
export function speedToPitchSlider(speedNorm: number): number {
  const n = Math.min(1, Math.max(0, speedNorm));
  return Math.round(n * 10000) / 100;
}

/** Map slider 0–100 to tempo fader position `0..1`. */
export function pitchSliderToSpeed(value: number): number {
  return Math.min(1, Math.max(0, value / 100));
}

/** Nudge tempo position by pitch percent (engine ±16% map). */
export function nudgeSpeed(speedNorm: number, deltaPercent: number): number {
  const ratio = normToSpeedRatio(speedNorm) + deltaPercent / 100;
  return speedRatioToNorm(ratio);
}

/** Mixxx-style pitch readout from tempo position (e.g. +0.00, -1.25). */
export function formatPitchOffset(speedNorm: number): string {
  const percent = (normToSpeedRatio(speedNorm) - 1) * 100;
  const sign = percent >= 0 ? "+" : "";
  return `${sign}${percent.toFixed(2)}`;
}

export function formatPitchPercent(speedNorm: number): string {
  return `${formatPitchOffset(speedNorm)}%`;
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
