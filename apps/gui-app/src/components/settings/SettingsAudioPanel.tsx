import { invoke } from "@tauri-apps/api/core";
import { useEffect, useMemo, useState } from "react";
import { Field, FieldDescription, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Slider, SliderValue } from "@/components/ui/slider";
import { Switch } from "@/components/ui/switch";
import { withForm } from "@/components/settings/form";
import { SettingsField, SettingsSectionHeader } from "@/components/settings/SettingsField";
import { SettingsSelect } from "@/components/settings/SettingsSelect";
import { settingsFormOptions } from "@/components/settings/settingsFormOptions";
import { MAX_TARGET_LUFS, MIN_TARGET_LUFS } from "@/lib/busSettings";
import {
  RESAMPLER_QUALITY_STEPS,
  resamplerQualityFromIndex,
  resamplerQualityIndex,
  resamplerQualityLabel,
} from "@/lib/resamplerQuality";
import type {
  AudioDeviceSummary,
  BusChannelMode,
  BusRouteSettings,
  SamplerBankInfo,
  SamplerPlayMode,
  SamplerStripRoute,
} from "@/types";
import { DeviceSelect } from "./DeviceSelect";

const BACKENDS = ["cpal", "auto", "null"] as const;

const BACKEND_OPTIONS = BACKENDS.map((backend) => ({
  value: backend,
  label: backend,
}));

const CHANNEL_MODE_OPTIONS: { value: BusChannelMode; label: string }[] = [
  { value: "stereo", label: "Stereo pair" },
  { value: "mono", label: "Mono (fold L+R)" },
];

const SAMPLER_PLAY_MODE_OPTIONS: { value: SamplerPlayMode; label: string }[] = [
  { value: "oneshot", label: "Oneshot" },
  { value: "hold", label: "Hold" },
  { value: "loop", label: "Loop" },
];

const SAMPLER_STRIP_ROUTE_OPTIONS: { value: SamplerStripRoute; label: string }[] = [
  { value: "before", label: "Before channel strip" },
  { value: "after", label: "After channel strip" },
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

export const SettingsAudioPanel = withForm({
  ...settingsFormOptions,
  props: {
    devices: [] as AudioDeviceSummary[],
    devicesLoading: false,
  },
  render: function Render({ form, devices, devicesLoading }) {
    const [banks, setBanks] = useState<SamplerBankInfo[]>([]);

    useEffect(() => {
      let cancelled = false;
      void invoke<SamplerBankInfo[]>("list_sampler_banks")
        .then((next) => {
          if (!cancelled) {
            setBanks(next);
          }
        })
        .catch(() => {
          if (!cancelled) {
            setBanks([]);
          }
        });
      return () => {
        cancelled = true;
      };
    }, []);

    const bankOptions = useMemo(
      () => [
        { value: "", label: "None" },
        ...banks.map((bank) => ({ value: bank.id, label: bank.name })),
      ],
      [banks],
    );

    return (
      <div className="space-y-8">
        <section className="space-y-5">
          <SettingsSectionHeader title="Engine" description="Output backend and buffering." />

          <form.AppField name="backend">
            {(field) => (
              <field.SelectField
                label="Audio backend"
                aria-label="Audio backend"
                options={[...BACKEND_OPTIONS]}
              />
            )}
          </form.AppField>

          <form.AppField name="sample_rate">
            {(field) => (
              <field.NumberField
                label="Sample rate (Hz)"
                aria-label="Sample rate (Hz)"
                min={8000}
                step={1000}
              />
            )}
          </form.AppField>

          <form.AppField name="buffer_size">
            {(field) => (
              <Field>
                <Slider
                  aria-label="Buffer size"
                  value={field.state.value}
                  min={BUFFER_SIZE_MIN}
                  max={BUFFER_SIZE_MAX}
                  step={BUFFER_SIZE_STEP}
                  onValueChange={(value) => {
                    const next = Array.isArray(value) ? value[0] : value;
                    if (next == null) {
                      return;
                    }
                    field.handleChange(snapBufferSize(next));
                  }}
                >
                  <div className="mb-2 flex items-center justify-between gap-1">
                    <FieldLabel className="font-medium text-sm">Buffer size (frames)</FieldLabel>
                    <SliderValue />
                  </div>
                </Slider>
                <FieldDescription>
                  Must be a multiple of {BUFFER_SIZE_STEP} frames (mixer graph chunk size).
                </FieldDescription>
              </Field>
            )}
          </form.AppField>

          <form.AppField name="low_latency">
            {(field) => <field.ToggleField label="Low latency mode" />}
          </form.AppField>
        </section>

        <section className="space-y-5 border-t border-white/8 pt-6">
          <SettingsSectionHeader
            title="Resample"
            description="Converts each track to what your audio device expects."
          />

          <form.AppField name="resampler_quality">
            {(field) => (
              <Field className="gap-1.5">
                <Slider
                  aria-label="Resampler quality"
                  value={resamplerQualityIndex(field.state.value)}
                  min={RESAMPLER_QUALITY_MIN}
                  max={RESAMPLER_QUALITY_MAX}
                  step={1}
                  onValueChange={(value) => {
                    const next = Array.isArray(value) ? value[0] : value;
                    if (next == null) {
                      return;
                    }
                    field.handleChange(resamplerQualityFromIndex(next));
                  }}
                >
                  <div className="mb-2 flex items-center justify-between gap-1">
                    <FieldLabel className="text-xs font-semibold uppercase tracking-wide text-zinc-500">
                      Resampler quality
                    </FieldLabel>
                    <span className="text-sm">{resamplerQualityLabel(field.state.value)}</span>
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
            )}
          </form.AppField>
        </section>

        <section className="space-y-5 border-t border-white/8 pt-6">
          <SettingsSectionHeader
            title="Normalization"
            description="Keep analyzed tracks near a consistent perceived loudness."
          />

          <form.AppField name="volume_normalizer_enabled">
            {(field) => <field.ToggleField label="Enable volume normalizer" />}
          </form.AppField>

          <form.Subscribe selector={(state) => state.values.volume_normalizer_enabled}>
            {(volumeNormalizerEnabled) => (
              <form.AppField name="target_lufs">
                {(field) => (
                  <field.SliderField
                    label="Target loudness"
                    aria-label="Target loudness"
                    disabled={!volumeNormalizerEnabled}
                    min={MIN_TARGET_LUFS}
                    max={MAX_TARGET_LUFS}
                    step={1}
                    valueLabel={`${field.state.value} LUFS`}
                    description="Lower values leave more headroom; −18 LUFS is recommended."
                  />
                )}
              </form.AppField>
            )}
          </form.Subscribe>
        </section>

        <section className="space-y-5 border-t border-white/8 pt-6">
          <SettingsSectionHeader
            title="Sampler"
            description="Default play mode for banks set to inherit, and default bank per deck."
          />

          <form.AppField name="sampler_play_mode">
            {(field) => (
              <field.SelectField
                label="Default play mode"
                aria-label="Sampler play mode"
                options={SAMPLER_PLAY_MODE_OPTIONS}
                hint="Used by banks whose play mode is Default (inherit)."
              />
            )}
          </form.AppField>

          <form.AppField name="sampler_strip_route">
            {(field) => (
              <field.SelectField
                label="Channel strip routing"
                aria-label="Sampler channel strip routing"
                options={SAMPLER_STRIP_ROUTE_OPTIONS}
                hint="Before runs pads through this deck's EQ, filter, and fader. After bypasses the strip. Takes effect the next time the engine starts."
              />
            )}
          </form.AppField>

          <form.Subscribe selector={(state) => state.values.deck_default_sampler_bank_id}>
            {(deckBanks) => (
              <>
                <SettingsField label="Deck A default bank">
                  <SettingsSelect
                    aria-label="Deck A default sampler bank"
                    value={deckBanks[0] ?? ""}
                    options={bankOptions}
                    onValueChange={(bankId) =>
                      form.setFieldValue("deck_default_sampler_bank_id", [
                        bankId || null,
                        deckBanks[1],
                      ])
                    }
                  />
                </SettingsField>

                <SettingsField label="Deck B default bank">
                  <SettingsSelect
                    aria-label="Deck B default sampler bank"
                    value={deckBanks[1] ?? ""}
                    options={bankOptions}
                    onValueChange={(bankId) =>
                      form.setFieldValue("deck_default_sampler_bank_id", [
                        deckBanks[0],
                        bankId || null,
                      ])
                    }
                  />
                </SettingsField>
              </>
            )}
          </form.Subscribe>
        </section>

        <section className="space-y-4 border-t border-white/8 pt-6">
          <SettingsSectionHeader
            title="Buses"
            description="Route master and optional preview output to devices. Mono mode folds stereo to one device channel."
          />

          <form.Subscribe selector={(state) => state.values.master_bus}>
            {(masterBus) => (
              <div className="space-y-4 rounded border border-white/10 bg-black/20 p-4">
                <p className="text-xs font-semibold uppercase tracking-wide text-zinc-500">
                  Master
                </p>
                <DeviceSelect
                  label="Output device"
                  value={masterBus.device_id}
                  devices={devices}
                  loading={devicesLoading}
                  onChange={(deviceId) =>
                    form.setFieldValue(
                      "master_bus",
                      updateBusRoute(masterBus, { device_id: deviceId }),
                    )
                  }
                />
                <BusChannelFields
                  route={masterBus}
                  defaultMonoChannel={1}
                  onChange={(next) => form.setFieldValue("master_bus", next)}
                />
              </div>
            )}
          </form.Subscribe>

          <div className="space-y-4 rounded border border-white/10 bg-black/20 p-4">
            <div className="flex items-center justify-between gap-3">
              <p className="text-xs font-semibold uppercase tracking-wide text-zinc-500">Preview</p>
              <form.AppField name="preview_enabled">
                {(field) => (
                  <Switch
                    checked={field.state.value}
                    aria-label="Enable preview bus"
                    onCheckedChange={(checked) => field.handleChange(checked)}
                  />
                )}
              </form.AppField>
            </div>
            <form.Subscribe
              selector={(state) => ({
                enabled: state.values.preview_enabled,
                previewBus: state.values.preview_bus,
              })}
            >
              {({ enabled, previewBus }) =>
                enabled ? (
                  <div className="space-y-4">
                    <DeviceSelect
                      label="Output device"
                      hint="Often headphones or a separate interface output."
                      value={previewBus.device_id}
                      devices={devices}
                      loading={devicesLoading}
                      onChange={(deviceId) =>
                        form.setFieldValue(
                          "preview_bus",
                          updateBusRoute(previewBus, { device_id: deviceId }),
                        )
                      }
                    />
                    <BusChannelFields
                      route={previewBus}
                      defaultMonoChannel={2}
                      onChange={(next) => form.setFieldValue("preview_bus", next)}
                    />
                  </div>
                ) : null
              }
            </form.Subscribe>
          </div>
        </section>
      </div>
    );
  },
});
