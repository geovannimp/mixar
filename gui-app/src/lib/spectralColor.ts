export const WAVEFORM_VISIBLE_SECS = 24;

const LOW_RGB: readonly [number, number, number] = [255, 72, 48];
const MID_RGB: readonly [number, number, number] = [118, 228, 88];
const HIGH_RGB: readonly [number, number, number] = [72, 188, 255];

export function spectralColor(low: number, mid: number, high: number): string {
  const total = low + mid + high + 1e-6;
  const l = low / total;
  const m = mid / total;
  const h = high / total;
  const r = l * LOW_RGB[0] + m * MID_RGB[0] + h * HIGH_RGB[0];
  const g = l * LOW_RGB[1] + m * MID_RGB[1] + h * HIGH_RGB[1];
  const b = l * LOW_RGB[2] + m * MID_RGB[2] + h * HIGH_RGB[2];
  return `rgb(${Math.round(r)}, ${Math.round(g)}, ${Math.round(b)})`;
}
