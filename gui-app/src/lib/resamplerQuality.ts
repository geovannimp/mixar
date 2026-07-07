export type ResamplerQuality = "low" | "medium" | "high";

export const RESAMPLER_QUALITY_STEPS: {
  value: ResamplerQuality;
  label: string;
}[] = [
  { value: "low", label: "Low" },
  { value: "medium", label: "Medium" },
  { value: "high", label: "High" },
];

export const DEFAULT_RESAMPLER_QUALITY: ResamplerQuality = "medium";

export function normalizeResamplerQuality(
  quality: string | null | undefined,
): ResamplerQuality {
  if (quality === "low" || quality === "high") {
    return quality;
  }
  return DEFAULT_RESAMPLER_QUALITY;
}

export function resamplerQualityIndex(quality: string | null | undefined): number {
  const normalized = normalizeResamplerQuality(quality);
  return RESAMPLER_QUALITY_STEPS.findIndex((step) => step.value === normalized);
}

export function resamplerQualityFromIndex(index: number): ResamplerQuality {
  const step =
    RESAMPLER_QUALITY_STEPS[
      Math.min(
        RESAMPLER_QUALITY_STEPS.length - 1,
        Math.max(0, Math.round(index)),
      )
    ];
  return step.value;
}

export function resamplerQualityLabel(quality: string | null | undefined): string {
  const normalized = normalizeResamplerQuality(quality);
  return (
    RESAMPLER_QUALITY_STEPS.find((step) => step.value === normalized)?.label ??
    "Medium"
  );
}
