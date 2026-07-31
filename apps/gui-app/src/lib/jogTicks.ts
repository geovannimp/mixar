/** Must match engine-dsp `JOG_INTERVALS_PER_REV`. */
export const JOG_INTERVALS_PER_REV = 720;

/** Convert an angular delta in degrees to relative jog ticks. */
export function degreesToJogTicks(deltaDeg: number): number {
  if (!Number.isFinite(deltaDeg) || deltaDeg === 0) {
    return 0;
  }
  return Math.round((deltaDeg / 360) * JOG_INTERVALS_PER_REV);
}
