import type { PadMode } from "../types";

export const PAD_MODES: readonly PadMode[] = [
  "hot_cue",
  "loop_roll",
  "beat_jump",
] as const;

export const PAD_MODE_LABELS: Record<PadMode, string> = {
  hot_cue: "Hot Cue",
  loop_roll: "Loop Roll",
  beat_jump: "Beat Jump",
};

export const PAD_MODE_SHORT_LABELS: Record<PadMode, string> = {
  hot_cue: "Cue",
  loop_roll: "Roll",
  beat_jump: "Jump",
};

export const LOOP_ROLL_BEATS = [1, 2, 4, 8, 16, 32, 64, 128] as const;
export const BEAT_JUMP_FORWARD = [1, 2, 4, 8, 16, 32, 64, 128] as const;
export const BEAT_JUMP_BACK = [-1, -2, -4, -8, -16, -32, -64, -128] as const;
