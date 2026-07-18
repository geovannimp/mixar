import { Field, FieldDescription, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Slider, SliderValue } from "@/components/ui/slider";
import { Switch } from "@/components/ui/switch";
import { MAX_TARGET_LUFS, MIN_TARGET_LUFS } from "@/lib/busSettings";
import {
  RESAMPLER_QUALITY_STEPS,
  resamplerQualityFromIndex,
  resamplerQualityIndex,
  resamplerQualityLabel,
} from "@/lib/resamplerQuality";
import type { AppSettings, AudioDeviceSummary, BusChannelMode, BusRouteSettings } from "@/types";
import { DeviceSelect } from "./DeviceSelect";
import { SettingsField, SettingsSectionHeader } from "./SettingsField";
import { SettingsSelect } from "./SettingsSelect";
import { SettingsToggle } from "./SettingsToggle";

const BACKENDS = ["cpal", "auto", "null"] as const;

const BACKEND_OPTIONS = BACKENDS.map((backend) => ({
  value: backend,
  label: backend,
}));

const CHANNEL_MODE_OPTIONS: { value: BusChannelMode; label: string }[] = [
  { value: "stereo", label: "Stereo pair" },
  { value: "mono", label: "Mono (fold L+R)" },
];

const RESAMPLER_QUALITY_MIN = 0;
const RESAMPLER_QUALITY_MAX = RESAMPLER_QUALITY_STEPS.length - 1;

const BUFFER_SIZE_MIN = 64;
const BUFFER_SIZE_MAX = 2048;
const BUFFER_SIZE_STEP = 64;

function snapBufferSize(value: number): number {
  const snapped = Math.round(value / BUFFER_SIZE_STEP) * BUFFER_SIZE_STEP;
  return Math.min(BUFFER_SIZE_MAX, Math.max(BUFFER_SIZE_MIN, snapped));
}

interface SettingsAudioPanelProps {
  draft: AppSettings;
  devices: AudioDeviceSummary[];
  devicesLoading: boolean;
  onChange: (next: AppSettings) => void;
}

function updateBusRoute(
  route: BusRouteSettings,
  patch: Partial<BusRouteSettings>,
): BusRouteSettings {
  return { ...route, ...patch };
}

function busMode(route: BusRouteSettings): BusChannelMode {
  return route.mode === "mono" ? "mono" : "stereo";
}

interface BusChannelFieldsProps {
  route: BusRouteSettings;
  /** Channel used when switching into mono mode. */
  defaultMonoChannel: number;
  onChange: (next: BusRouteSettings) => void;
}

function BusChannelFields({ route, defaultMonoChannel, onChange }: BusChannelFieldsProps) {
  const mode = busMode(route);

  return (
    <>
      <SettingsField label="Channel mode">
        <SettingsSelect
          aria-label="Channel mode"
          value={mode}
          options={CHANNEL_MODE_OPTIONS}
          onValueChange={(selected) => {
            if (selected === "mono") {
              onChange(
                updateBusRoute(route, {
                  mode: "mono",
                  left_channel: defaultMonoChannel,
                  right_channel: defaultMonoChannel,
                }),
              );
              return;
            }
            onChange(
              updateBusRoute(route, {
                mode: "stereo",
                right_channel:
                  route.right_channel === route.left_channel
                    ? route.left_channel + 1
                    : route.right_channel,
              }),
            );
          }}
        />
      </SettingsField>
      {mode === "mono" ? (
        <SettingsField label="Mono channel">
          <Input
            type="number"
            min={1}
            value={route.left_channel}
            onChange={(event) => {
              const channel = Number(event.target.value) || 1;
              onChange(
                updateBusRoute(route, {
                  left_channel: channel,
                  right_channel: channel,
                  mode: "mono",
                }),
              );
            }}
          />
        </SettingsField>
      ) : (
        <div className="grid grid-cols-2 gap-3">
          <SettingsField label="Left channel">
            <Input
              type="number"
              min={1}
              value={route.left_channel}
              onChange={(event) =>
                onChange(
                  updateBusRoute(route, {
                    left_channel: Number(event.target.value) || 1,
                    mode: "stereo",
                  }),
                )
              }
            />
          </SettingsField>
          <SettingsField label="Right channel">
            <Input
              type="number"
              min={1}
              value={route.right_channel}
              onChange={(event) =>
                onChange(
                  updateBusRoute(route, {
                    right_channel: Number(event.target.value) || 2,
                    mode: "stereo",
                  }),
                )
              }
            />
          </SettingsField>
        </div>
      )}
    </>
  );
}

export function SettingsAudioPanel({
  draft,
  devices,
  devicesLoading,
  onChange,
}: SettingsAudioPanelProps) {
  return (
    <div className="space-y-8">
      <section className="space-y-5">
        <SettingsSectionHeader title="Engine" description="Output backend and buffering." />

        <SettingsField label="Audio backend">
          <SettingsSelect
            aria-label="Audio backend"
            value={draft.backend}
            options={BACKEND_OPTIONS}
            onValueChange={(backend) => onChange({ ...draft, backend })}
          />
        </SettingsField>

        <SettingsField label="Sample rate (Hz)">
          <Input
            type="number"
            min={8000}
            step={1000}
            value={draft.sample_rate}
            onChange={(event) =>
              onChange({
                ...draft,
                sample_rate: Number(event.target.value) || draft.sample_rate,
              })
            }
          />
        </SettingsField>

        <Field>
          <Slider
            aria-label="Buffer size"
            value={draft.buffer_size}
            min={BUFFER_SIZE_MIN}
            max={BUFFER_SIZE_MAX}
            step={BUFFER_SIZE_STEP}
            onValueChange={(value) => {
              const next = Array.isArray(value) ? value[0] : value;
              if (next == null) {
                return;
              }
              onChange({
                ...draft,
                buffer_size: snapBufferSize(next),
              });
            }}
          >
            <div className="mb-2 flex items-center justify-between gap-1">
              <FieldLabel className="font-medium text-sm">Buffer size (frames)</FieldLabel>
              <SliderValue />
            </div>
          </Slider>
        </Field>

        <SettingsToggle
          label="Low latency mode"
          checked={draft.low_latency}
          onCheckedChange={(low_latency) => onChange({ ...draft, low_latency })}
        />
      </section>

      <section className="space-y-5 border-t border-white/8 pt-6">
        <SettingsSectionHeader
          title="Resample"
          description="Converts each track to what your audio device expects."
        />

        <Field className="gap-1.5">
          <Slider
            aria-label="Resampler quality"
            value={resamplerQualityIndex(draft.resampler_quality)}
            min={RESAMPLER_QUALITY_MIN}
            max={RESAMPLER_QUALITY_MAX}
            step={1}
            onValueChange={(value) => {
              const next = Array.isArray(value) ? value[0] : value;
              if (next == null) {
                return;
              }
              onChange({
                ...draft,
                resampler_quality: resamplerQualityFromIndex(next),
              });
            }}
          >
            <div className="mb-2 flex items-center justify-between gap-1">
              <FieldLabel className="text-xs font-semibold uppercase tracking-wide text-zinc-500">
                Resampler quality
              </FieldLabel>
              <span className="text-sm">{resamplerQualityLabel(draft.resampler_quality)}</span>
            </div>
          </Slider>
          <div
            aria-label="Resampler quality levels"
            className="mt-1 flex w-full items-center justify-between gap-1 px-2.5 font-medium text-muted-foreground text-xs"
            role="group"
          >
            {RESAMPLER_QUALITY_STEPS.map((step) => (
              <span
                className="flex w-0 flex-col items-center justify-center gap-2"
                key={step.value}
              >
                <span className="h-1 w-px bg-muted-foreground/72" />
                <span>{step.label}</span>
              </span>
            ))}
          </div>
          <FieldDescription>Higher quality uses more CPU.</FieldDescription>
        </Field>
      </section>

      <section className="space-y-5 border-t border-white/8 pt-6">
        <SettingsSectionHeader
          title="Normalization"
          description="Keep analyzed tracks near a consistent perceived loudness."
        />

        <SettingsToggle
          label="Enable volume normalizer"
          checked={draft.volume_normalizer_enabled}
          onCheckedChange={(volume_normalizer_enabled) =>
            onChange({ ...draft, volume_normalizer_enabled })
          }
        />

        <Field>
          <Slider
            aria-label="Target loudness"
            disabled={!draft.volume_normalizer_enabled}
            value={draft.target_lufs}
            min={MIN_TARGET_LUFS}
            max={MAX_TARGET_LUFS}
            step={1}
            onValueChange={(value) => {
              const target_lufs = Array.isArray(value) ? value[0] : value;
              if (target_lufs == null) {
                return;
              }
              onChange({ ...draft, target_lufs });
            }}
          >
            <div className="mb-2 flex items-center justify-between gap-1">
              <FieldLabel className="font-medium text-sm">Target loudness</FieldLabel>
              <span className="text-sm">{draft.target_lufs} LUFS</span>
            </div>
          </Slider>
          <FieldDescription>
            Lower values leave more headroom; −18 LUFS is recommended.
          </FieldDescription>
        </Field>
      </section>

      <section className="space-y-4 border-t border-white/8 pt-6">
        <SettingsSectionHeader
          title="Buses"
          description="Route master and optional preview output to devices. Mono mode folds stereo to one device channel."
        />

        <div className="space-y-4 rounded border border-white/10 bg-black/20 p-4">
          <p className="text-xs font-semibold uppercase tracking-wide text-zinc-500">Master</p>
          <DeviceSelect
            label="Output device"
            value={draft.master_bus.device_id}
            devices={devices}
            loading={devicesLoading}
            onChange={(deviceId) =>
              onChange({
                ...draft,
                master_bus: updateBusRoute(draft.master_bus, {
                  device_id: deviceId,
                }),
              })
            }
          />
          <BusChannelFields
            route={draft.master_bus}
            defaultMonoChannel={1}
            onChange={(master_bus) => onChange({ ...draft, master_bus })}
          />
        </div>

        <div className="space-y-4 rounded border border-white/10 bg-black/20 p-4">
          <div className="flex items-center justify-between gap-3">
            <p className="text-xs font-semibold uppercase tracking-wide text-zinc-500">Preview</p>
            <Switch
              checked={draft.preview_enabled}
              aria-label="Enable preview bus"
              onCheckedChange={(preview_enabled) => onChange({ ...draft, preview_enabled })}
            />
          </div>
          {draft.preview_enabled ? (
            <div className="space-y-4">
              <DeviceSelect
                label="Output device"
                hint="Often headphones or a separate interface output."
                value={draft.preview_bus.device_id}
                devices={devices}
                loading={devicesLoading}
                onChange={(deviceId) =>
                  onChange({
                    ...draft,
                    preview_bus: updateBusRoute(draft.preview_bus, {
                      device_id: deviceId,
                    }),
                  })
                }
              />
              <BusChannelFields
                route={draft.preview_bus}
                defaultMonoChannel={2}
                onChange={(preview_bus) => onChange({ ...draft, preview_bus })}
              />
            </div>
          ) : null}
        </div>
      </section>
    </div>
  );
}
