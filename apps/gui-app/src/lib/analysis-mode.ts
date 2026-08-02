export type AnalysisMode = "fast" | "precise" | "complete";

export type AnalysisModeOption = {
  label: string;
  description: string;
  value: AnalysisMode;
};

export const ANALYSIS_MODE_OPTIONS: AnalysisModeOption[] = [
  {
    value: "fast",
    label: "Fast",
    description: "First 30 seconds — quick BPM and key estimate.",
  },
  {
    value: "precise",
    label: "Precise",
    description: "First half of the track — better accuracy on long songs.",
  },
  {
    value: "complete",
    label: "Complete",
    description: "Full track — slowest, most thorough analysis.",
  },
];

export function findAnalysisModeOption(value: AnalysisMode): AnalysisModeOption {
  const option = ANALYSIS_MODE_OPTIONS.find((item) => item.value === value);
  if (!option) {
    throw new Error(`Unknown analysis mode: ${value}`);
  }
  return option;
}
