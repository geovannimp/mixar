import type { PadMode } from "@/types";

export const PAD_MODES: readonly PadMode[] = [
  "hot_cue",
  "loop_roll",
  "beat_jump",
  "sampler",
] as const;

export function cyclePadMode(mode: PadMode, direction: number): PadMode {
  const index = PAD_MODES.indexOf(mode);
  const current = index >= 0 ? index : 0;
  const len = PAD_MODES.length;
  const next = direction < 0 ? (current + len - 1) % len : (current + 1) % len;
  return PAD_MODES[next] ?? "hot_cue";
}

export const PAD_MODE_LABELS: Record<PadMode, string> = {
  hot_cue: "Hot Cue",
  loop_roll: "Loop Roll",
  beat_jump: "Beat Jump",
  sampler: "Sampler",
};

export const PAD_MODE_SHORT_LABELS: Record<PadMode, string> = {
  hot_cue: "Cue",
  loop_roll: "Roll",
  beat_jump: "Jump",
  sampler: "Sample",
};

export const LOOP_ROLL_BEATS = [1 / 32, 1 / 16, 1 / 8, 1 / 4, 1 / 2, 1, 2, 4] as const;
export const BEAT_JUMP_FORWARD = [1, 2, 4, 8, 16, 32, 64, 128] as const;
export const BEAT_JUMP_BACK = [-1, -2, -4, -8, -16, -32, -64, -128] as const;

export function formatBeatLength(beats: number): string {
  if (beats >= 1 || beats <= 0) {
    return String(beats);
  }
  return `1/${Math.round(1 / beats)}`;
}
