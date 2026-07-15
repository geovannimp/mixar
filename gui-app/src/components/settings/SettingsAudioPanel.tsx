import { Field, FieldDescription, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Slider, SliderValue } from "@/components/ui/slider";
import {
  RESAMPLER_QUALITY_STEPS,
  resamplerQualityFromIndex,
  resamplerQualityIndex,
  resamplerQualityLabel,
} from "@/lib/resamplerQuality";
import type {
  AppSettings,
  AudioDeviceSummary,
  BusChannelMode,
  BusRouteSettings,
} from "@/types";
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
  onChange: (next: BusRouteSettings) => void;
}

function BusChannelFields({ route, onChange }: BusChannelFieldsProps) {
  const mode = busMode(route);

  return (
    <>
      <SettingsField label="Channel mode">
        <SettingsSelect
          aria-label="Channel mode"
          value={mode}
          options={CHANNEL_MODE_OPTIONS}
          onValueChange={(selected) => {
            onChange(
              updateBusRoute(route, {
                mode: selected,
                ...(selected === "mono"
                  ? { right_channel: route.left_channel }
                  : {
                      right_channel:
                        route.right_channel === route.left_channel
                          ? route.left_channel + 1
                          : route.right_channel,
                    }),
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
        <SettingsSectionHeader
          title="Engine"
          description="Output backend and buffering."
        />

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
          />
          <div className="flex items-center justify-between gap-2">
            <FieldLabel>Buffer size (frames)</FieldLabel>
            <SliderValue>{draft.buffer_size}</SliderValue>
          </div>
          <FieldDescription>
            Smaller buffers reduce latency and raise CPU / drop risk.
          </FieldDescription>
        </Field>

        <SettingsToggle
          label="Low latency"
          checked={draft.low_latency}
          onCheckedChange={(low_latency) => onChange({ ...draft, low_latency })}
        />

        <Field>
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
          />
          <div className="flex items-center justify-between gap-2">
            <FieldLabel>Resampler quality</FieldLabel>
            <SliderValue>
              {resamplerQualityLabel(draft.resampler_quality)}
            </SliderValue>
          </div>
          <FieldDescription>Higher quality uses more CPU.</FieldDescription>
        </Field>
      </section>

      <section className="space-y-4 border-t border-white/8 pt-6">
        <SettingsSectionHeader
          title="Buses"
          description="Route master and optional preview output to devices. Mono mode folds stereo to one device channel."
        />

        <div className="space-y-4 rounded border border-white/10 bg-black/20 p-4">
          <p className="text-xs font-semibold uppercase tracking-wide text-zinc-500">
            Master
          </p>
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
            onChange={(master_bus) => onChange({ ...draft, master_bus })}
          />
        </div>

        <SettingsToggle
          label="Enable preview bus"
          checked={draft.preview_enabled}
          onCheckedChange={(preview_enabled) =>
            onChange({ ...draft, preview_enabled })
          }
        />

        {draft.preview_enabled && (
          <div className="space-y-4 rounded border border-white/10 bg-black/20 p-4">
            <p className="text-xs font-semibold uppercase tracking-wide text-zinc-500">
              Preview
            </p>
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
              onChange={(preview_bus) => onChange({ ...draft, preview_bus })}
            />
          </div>
        )}
      </section>
    </div>
  );
}
