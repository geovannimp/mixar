import { formOptions } from "@tanstack/react-form";
import {
  DEFAULT_MASTER_BUS,
  DEFAULT_PREVIEW_BUS,
  DEFAULT_SAMPLER_PLAY_MODE,
  DEFAULT_SAMPLER_STRIP_ROUTE,
  DEFAULT_TARGET_LUFS,
  DEFAULT_VOLUME_NORMALIZER_ENABLED,
} from "@/lib/busSettings";
import { DEFAULT_LIBRARY_TABLE_COLUMNS } from "@/lib/libraryTable";
import type { AppSettings } from "@/types";

/** Type-only defaults for `withForm` / `formOptions`; page supplies real values at runtime. */
export const settingsFormDefaultValues: AppSettings = {
  backend: "cpal",
  sample_rate: 48_000,
  buffer_size: 512,
  low_latency: false,
  resampler_quality: "medium",
  master_bus: DEFAULT_MASTER_BUS,
  preview_enabled: false,
  preview_bus: DEFAULT_PREVIEW_BUS,
  analysis_duration: "precise",
  scan_folder_tree: true,
  library_table_columns: DEFAULT_LIBRARY_TABLE_COLUMNS,
  volume_normalizer_enabled: DEFAULT_VOLUME_NORMALIZER_ENABLED,
  target_lufs: DEFAULT_TARGET_LUFS,
  sampler_play_mode: DEFAULT_SAMPLER_PLAY_MODE,
  sampler_strip_route: DEFAULT_SAMPLER_STRIP_ROUTE,
  deck_default_sampler_bank_id: [null, null],
};

export const settingsFormOptions = formOptions({
  defaultValues: settingsFormDefaultValues,
});
