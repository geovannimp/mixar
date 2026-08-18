/** MessagePack wire codec matching `crates/engine-api` (named maps + Zod codecs). */

import { decode, encode } from "@msgpack/msgpack";
import { z } from "zod";

export const KindSchema = z.enum([
  "play",
  "pause",
  "toggle_play",
  "seek",
  "set_volume",
  "set_eq",
  "set_eq_band",
  "set_speed",
  "set_tempo_range",
  "set_filter",
  "set_gain_trim",
  "set_headphone_cue",
  "toggle_headphone_cue",
  "set_crossfader",
  "set_cue_mix",
  "set_master_cue",
  "toggle_master_cue",
  "toggle_sync",
  "set_master_deck",
  "unload",
  "set_cue_point",
  "begin_cue_hold",
  "end_cue_hold",
  "set_quantize",
  "toggle_quantize",
  "set_auto_loop",
  "loop_in",
  "loop_out",
  "exit_loop",
  "beat_jump",
  "set_pad_mode",
  "begin_loop_roll",
  "end_loop_roll",
  "pad_press",
  "pad_release",
  "hot_cue_pad_press",
  "hot_cue_pad_release",
  "loop_roll_pad_press",
  "loop_roll_pad_release",
  "beat_jump_pad_press",
  "beat_jump_pad_release",
  "sampler_pad_press",
  "sampler_pad_release",
  "trigger_hot_cue",
  "recall_saved_loop",
  "assign_sampler",
  "assign_sampler_track",
  "clear_sampler",
  "set_sampler_bank",
  "create_sampler_bank",
  "update_sampler_bank",
  "delete_sampler_bank",
  "save_hot_cue",
  "delete_hot_cue",
  "save_loop",
  "delete_loop",
  "load_path",
  "load_library_track",
  "jog_touch",
  "jog_turn",
  "set_jog_mode",
  "start_engine",
  "updated",
  "position",
  "levels",
  "status",
  "error",
  "notice",
]);
export type Kind = z.infer<typeof KindSchema>;

export const EqBandSchema = z.enum(["low", "mid", "high"]);
export type EqBand = z.infer<typeof EqBandSchema>;

export const OriginSchema = z.union([
  z.literal("engine"),
  z.literal("mixer"),
  z.object({ deck: z.number().int().nonnegative() }),
]);
export type Origin = z.infer<typeof OriginSchema>;

export const DeckEqSchema = z.object({
  low: z.number().min(0).max(1),
  mid: z.number().min(0).max(1),
  high: z.number().min(0).max(1),
});
export type DeckEq = z.infer<typeof DeckEqSchema>;

/** Absolute control position on the wire (`0..1`). */
const UnitNorm = z.number().min(0).max(1);

export const SyncModeSchema = z.enum(["off", "tempo", "beat"]);
export type SyncMode = z.infer<typeof SyncModeSchema>;

export const PadModeSchema = z.enum(["hot_cue", "loop_roll", "beat_jump", "sampler"]);
export type PadMode = z.infer<typeof PadModeSchema>;

export const JogModeSchema = z.enum(["vinyl", "pitch_bend", "ignore"]);
export type JogMode = z.infer<typeof JogModeSchema>;

export const LoopRegionSchema = z.object({
  in_ms: z.number(),
  out_ms: z.number(),
  active: z.boolean(),
});
export type LoopRegion = z.infer<typeof LoopRegionSchema>;

export const DeckHotCueSchema = z.object({
  slot: z.number().int().nonnegative(),
  position_ms: z.number(),
  loop_length_beats: z.number().int().nullable().optional(),
  color: z.string().nullable().optional(),
  label: z.string().nullable().optional(),
});
export type DeckHotCue = z.infer<typeof DeckHotCueSchema>;

export const DeckSavedLoopSchema = z.object({
  slot: z.number().int().nonnegative(),
  in_ms: z.number(),
  out_ms: z.number(),
  label: z.string().nullable().optional(),
  color: z.string().nullable().optional(),
});
export type DeckSavedLoop = z.infer<typeof DeckSavedLoopSchema>;

export const SamplerPlayModeSchema = z.enum(["oneshot", "hold", "loop"]);
export type SamplerPlayMode = z.infer<typeof SamplerPlayModeSchema>;

export const SamplerSlotInfoSchema = z.object({
  label: z.string().nullable(),
  track_id: z.string().nullable(),
  path: z.string().nullable(),
  duration_ms: z.number().nullable(),
});
export type SamplerSlotInfo = z.infer<typeof SamplerSlotInfoSchema>;

export const SamplerBankInfoSchema = z.object({
  id: z.string(),
  name: z.string(),
  play_mode: SamplerPlayModeSchema.nullable(),
  sort_index: z.number().int(),
});
export type SamplerBankInfo = z.infer<typeof SamplerBankInfoSchema>;

export const SamplerStatusSchema = z.object({
  banks: z.array(SamplerBankInfoSchema),
  active_bank_id: z.string().nullable(),
  active_bank_name: z.string().nullable(),
  bank_play_mode: SamplerPlayModeSchema.nullable(),
  deck_slots: z.array(z.array(SamplerSlotInfoSchema)),
  effective_play_modes: z.array(SamplerPlayModeSchema),
});
export type SamplerStatus = z.infer<typeof SamplerStatusSchema>;

export const DeckSnapshotSchema = z.object({
  id: z.number().int().nonnegative(),
  track: z.string().nullable(),
  track_id: z.string().nullable(),
  title: z.string().nullable(),
  artist: z.string().nullable(),
  bpm: z.number().nullable(),
  key: z.string().nullable(),
  playing: z.boolean(),
  volume: UnitNorm,
  speed: UnitNorm,
  tempo_range: z.number().positive(),
  eq: DeckEqSchema,
  filter: UnitNorm,
  gain_trim: UnitNorm,
  headphone_cue: z.boolean(),
  sync_mode: SyncModeSchema,
  cue_point_ms: z.number().nullable(),
  quantize: z.boolean(),
  active_loop: LoopRegionSchema.nullable(),
  pad_mode: PadModeSchema,
  position_ms: z.number().nullable(),
  duration_ms: z.number().nullable(),
  hot_cues: z.array(DeckHotCueSchema),
  saved_loops: z.array(DeckSavedLoopSchema),
  loudness_lufs: z.number().nullable(),
  auto_gain_db: z.number(),
  active_sampler_bank_id: z.string().nullable(),
  top_jog_mode: JogModeSchema,
  outer_jog_mode: JogModeSchema,
  jog_touching: z.boolean(),
});
export type DeckSnapshot = z.infer<typeof DeckSnapshotSchema>;

export const EngineStatusPayloadSchema = z.object({
  running: z.boolean(),
  sample_rate: z.number().int().nonnegative(),
  crossfader: UnitNorm,
  cue_mix: UnitNorm,
  master_cue: z.boolean(),
  master_deck: z.number().int().nonnegative(),
  decks: z.array(DeckSnapshotSchema),
  sampler: SamplerStatusSchema,
});
export type EngineStatusPayload = z.infer<typeof EngineStatusPayloadSchema>;

/** Absolute MIDI soft-takeover flag; omit for UI hard sets (Rust `#[serde(default)]`). */
const SoftTakeoverField = {
  soft_takeover: z.boolean().optional(),
};

export const CmdBodySchema = z.discriminatedUnion("type", [
  z.object({ type: z.literal("empty") }),
  z.object({ type: z.literal("seek"), position_ms: z.number() }),
  z.object({ type: z.literal("set_volume"), volume: UnitNorm, ...SoftTakeoverField }),
  z.object({
    type: z.literal("set_eq"),
    low: UnitNorm,
    mid: UnitNorm,
    high: UnitNorm,
  }),
  z.object({
    type: z.literal("set_eq_band"),
    band: EqBandSchema,
    gain: UnitNorm,
    ...SoftTakeoverField,
  }),
  z.object({ type: z.literal("set_speed"), speed: UnitNorm, ...SoftTakeoverField }),
  z.object({ type: z.literal("set_tempo_range"), tempo_range: z.number().positive() }),
  z.object({ type: z.literal("set_filter"), filter: UnitNorm, ...SoftTakeoverField }),
  z.object({ type: z.literal("set_gain_trim"), gain_trim: UnitNorm, ...SoftTakeoverField }),
  z.object({ type: z.literal("set_headphone_cue"), enabled: z.boolean() }),
  z.object({ type: z.literal("set_crossfader"), position: UnitNorm, ...SoftTakeoverField }),
  z.object({ type: z.literal("set_cue_mix"), mix: UnitNorm, ...SoftTakeoverField }),
  z.object({ type: z.literal("set_master_cue"), enabled: z.boolean() }),
  z.object({ type: z.literal("toggle_sync"), beat_sync: z.boolean() }),
  z.object({ type: z.literal("set_quantize"), enabled: z.boolean() }),
  z.object({
    type: z.literal("set_auto_loop"),
    beats: z.number().finite().positive(),
  }),
  z.object({
    type: z.literal("beat_jump"),
    beats: z
      .number()
      .finite()
      .refine((n) => n !== 0, { message: "beats must be non-zero" }),
  }),
  z.object({ type: z.literal("set_pad_mode"), mode: PadModeSchema }),
  z.object({
    type: z.literal("begin_loop_roll"),
    beats: z.number().finite().positive(),
  }),
  z.object({
    type: z.literal("pad_press"),
    slot: z.number().int().nonnegative(),
    shift: z.boolean().optional(),
  }),
  z.object({ type: z.literal("pad_release"), slot: z.number().int().nonnegative() }),
  z.object({
    type: z.literal("hot_cue_pad_press"),
    slot: z.number().int().nonnegative(),
    shift: z.boolean().optional(),
  }),
  z.object({ type: z.literal("hot_cue_pad_release"), slot: z.number().int().nonnegative() }),
  z.object({ type: z.literal("loop_roll_pad_press"), slot: z.number().int().nonnegative() }),
  z.object({ type: z.literal("loop_roll_pad_release"), slot: z.number().int().nonnegative() }),
  z.object({ type: z.literal("beat_jump_pad_press"), slot: z.number().int().nonnegative() }),
  z.object({ type: z.literal("beat_jump_pad_release"), slot: z.number().int().nonnegative() }),
  z.object({
    type: z.literal("sampler_pad_press"),
    slot: z.number().int().nonnegative(),
    shift: z.boolean().optional(),
  }),
  z.object({ type: z.literal("sampler_pad_release"), slot: z.number().int().nonnegative() }),
  z.object({ type: z.literal("trigger_hot_cue"), position_ms: z.number() }),
  z.object({
    type: z.literal("recall_saved_loop"),
    in_ms: z.number(),
    out_ms: z.number(),
  }),
  z.object({
    type: z.literal("assign_sampler"),
    slot: z.number().int().nonnegative(),
    path: z.string(),
  }),
  z.object({
    type: z.literal("assign_sampler_track"),
    slot: z.number().int().nonnegative(),
    track_id: z.string(),
  }),
  z.object({ type: z.literal("clear_sampler"), slot: z.number().int().nonnegative() }),
  z.object({ type: z.literal("set_sampler_bank"), bank_id: z.string() }),
  z.object({
    type: z.literal("create_sampler_bank"),
    name: z.string().nullable().optional(),
    play_mode: z.enum(["oneshot", "hold", "loop"]).nullable().optional(),
  }),
  z.object({
    type: z.literal("update_sampler_bank"),
    bank_id: z.string(),
    name: z.string(),
    play_mode: z.enum(["oneshot", "hold", "loop"]).nullable().optional(),
  }),
  z.object({ type: z.literal("delete_sampler_bank"), bank_id: z.string() }),
  z.object({ type: z.literal("save_hot_cue"), slot: z.number().int().nonnegative() }),
  z.object({ type: z.literal("delete_hot_cue"), slot: z.number().int().nonnegative() }),
  z.object({ type: z.literal("save_loop"), slot: z.number().int().nonnegative() }),
  z.object({ type: z.literal("delete_loop"), slot: z.number().int().nonnegative() }),
  z.object({ type: z.literal("load_path"), path: z.string() }),
  z.object({ type: z.literal("load_library_track"), track_id: z.string() }),
  z.object({ type: z.literal("jog_touch"), touching: z.boolean() }),
  z.object({ type: z.literal("jog_turn"), delta: z.number().int() }),
  z.object({
    type: z.literal("set_jog_mode"),
    top: JogModeSchema,
    outer: JogModeSchema,
  }),
]);
export type CmdBody = z.infer<typeof CmdBodySchema>;

export const EvtBodySchema = z.discriminatedUnion("type", [
  z.object({ type: z.literal("empty") }),
  z.object({
    type: z.literal("deck_updated"),
    id: z.number().int().nonnegative(),
    track: z.string().nullable(),
    track_id: z.string().nullable(),
    title: z.string().nullable(),
    artist: z.string().nullable(),
    bpm: z.number().nullable(),
    key: z.string().nullable(),
    playing: z.boolean(),
    volume: UnitNorm,
    speed: UnitNorm,
    tempo_range: z.number().positive(),
    eq: DeckEqSchema,
    filter: UnitNorm,
    gain_trim: UnitNorm,
    headphone_cue: z.boolean(),
    sync_mode: SyncModeSchema,
    cue_point_ms: z.number().nullable(),
    quantize: z.boolean(),
    active_loop: LoopRegionSchema.nullable(),
    pad_mode: PadModeSchema,
    position_ms: z.number().nullable(),
    duration_ms: z.number().nullable(),
    hot_cues: z.array(DeckHotCueSchema),
    saved_loops: z.array(DeckSavedLoopSchema),
    loudness_lufs: z.number().nullable(),
    auto_gain_db: z.number(),
    active_sampler_bank_id: z.string().nullable(),
    top_jog_mode: JogModeSchema,
    outer_jog_mode: JogModeSchema,
    jog_touching: z.boolean(),
  }),
  z.object({ type: z.literal("position"), position_ms: z.number() }),

  z.object({
    type: z.literal("levels"),
    peak_l: z.number(),
    peak_r: z.number(),
    peak_hold_l: z.number(),
    peak_hold_r: z.number(),
  }),
  z.object({ type: z.literal("engine_status"), status: EngineStatusPayloadSchema }),
  z.object({ type: z.literal("error"), message: z.string() }),
  z.object({ type: z.literal("notice"), message: z.string() }),
]);
export type EvtBody = z.infer<typeof EvtBodySchema>;

export const WireMessageSchema = z.object({
  origin: OriginSchema,
  kind: KindSchema,
  revision: z.number().int().nonnegative(),
  action_timestamp_ms: z.number().int().nonnegative().default(0),
  body: z.custom<Uint8Array>((val): val is Uint8Array => val instanceof Uint8Array),
});
export type WireMessage = z.infer<typeof WireMessageSchema>;

const BytesSchema = z.custom<Uint8Array>((val): val is Uint8Array => val instanceof Uint8Array);

function asUint8Array(value: Uint8Array): Uint8Array {
  return value instanceof Uint8Array ? value : new Uint8Array(value);
}

/** Bytes ↔ WireMessage (named MessagePack map). */
export const WireMessageCodec = z.codec(BytesSchema, WireMessageSchema, {
  decode: (bytes) => {
    const raw = decode(bytes) as Record<string, unknown>;
    return WireMessageSchema.parse({
      ...raw,
      body: asUint8Array(raw.body as Uint8Array),
    });
  },
  encode: (message) => encode(WireMessageSchema.parse(message)),
});

/** Bytes ↔ CmdBody. */
export const CmdBodyCodec = z.codec(BytesSchema, CmdBodySchema, {
  decode: (bytes) => CmdBodySchema.parse(decode(bytes)),
  encode: (body) => encode(CmdBodySchema.parse(body)),
});

/** Bytes ↔ EvtBody. */
export const EvtBodyCodec = z.codec(BytesSchema, EvtBodySchema, {
  decode: (bytes) => EvtBodySchema.parse(decode(bytes)),
  encode: (body) => encode(EvtBodySchema.parse(body)),
});

export function decodeWire(bytes: Uint8Array): WireMessage {
  return WireMessageCodec.decode(bytes);
}

export function encodeWire(message: WireMessage): Uint8Array {
  return asUint8Array(WireMessageCodec.encode(message));
}

export function decodeCmdBody(bytes: Uint8Array): CmdBody {
  return CmdBodyCodec.decode(bytes);
}

export function encodeCmdBody(body: CmdBody): Uint8Array {
  return asUint8Array(CmdBodyCodec.encode(body));
}

export function decodeEvtBody(bytes: Uint8Array): EvtBody {
  return EvtBodyCodec.decode(bytes);
}

export function encodeEvtBody(body: EvtBody): Uint8Array {
  return asUint8Array(EvtBodyCodec.encode(body));
}

export function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
}

export function hexToBytes(hex: string): Uint8Array {
  const trimmed = hex.trim();
  const out = new Uint8Array(trimmed.length / 2);
  for (let i = 0; i < out.length; i += 1) {
    out[i] = Number.parseInt(trimmed.slice(i * 2, i * 2 + 2), 16);
  }
  return out;
}

export function encodeWireCmd(
  origin: Origin,
  kind: Kind,
  body: CmdBody,
  revision = 0,
  actionTimestampMs = 0,
): Uint8Array {
  return encodeWire({
    origin,
    kind,
    revision,
    action_timestamp_ms: actionTimestampMs,
    body: encodeCmdBody(body),
  });
}

/** Cmd kinds accepted by `EngineTransport.publish` (subset of wire `Kind`). */
export type CmdKind =
  | "play"
  | "pause"
  | "toggle_play"
  | "seek"
  | "set_volume"
  | "set_eq"
  | "set_eq_band"
  | "set_speed"
  | "set_tempo_range"
  | "set_filter"
  | "set_gain_trim"
  | "set_headphone_cue"
  | "toggle_headphone_cue"
  | "set_crossfader"
  | "set_cue_mix"
  | "set_master_cue"
  | "toggle_master_cue"
  | "toggle_sync"
  | "set_master_deck"
  | "unload"
  | "set_cue_point"
  | "begin_cue_hold"
  | "end_cue_hold"
  | "set_quantize"
  | "toggle_quantize"
  | "set_auto_loop"
  | "loop_in"
  | "loop_out"
  | "exit_loop"
  | "beat_jump"
  | "set_pad_mode"
  | "begin_loop_roll"
  | "end_loop_roll"
  | "pad_press"
  | "pad_release"
  | "sampler_pad_press"
  | "sampler_pad_release"
  | "trigger_hot_cue"
  | "recall_saved_loop"
  | "assign_sampler"
  | "assign_sampler_track"
  | "clear_sampler"
  | "set_sampler_bank"
  | "create_sampler_bank"
  | "update_sampler_bank"
  | "delete_sampler_bank"
  | "save_hot_cue"
  | "delete_hot_cue"
  | "save_loop"
  | "delete_loop"
  | "load_path"
  | "load_library_track"
  | "jog_touch"
  | "jog_turn"
  | "set_jog_mode"
  | "start_engine";

/** Nested CmdBody: no fields → empty; otherwise tag with `kind`. Strips wire-only `action_timestamp_ms`. */
export function cmdBodyForKind(kind: CmdKind, fields: Record<string, unknown> = {}): CmdBody {
  const { action_timestamp_ms: _actionTimestampMs, ...bodyFields } = fields;
  if (Object.keys(bodyFields).length === 0) {
    return { type: "empty" };
  }
  return CmdBodySchema.parse({ type: kind, ...bodyFields });
}

export function actionTimestampMsFromFields(fields: Record<string, unknown> = {}): number {
  const value = fields.action_timestamp_ms;
  return typeof value === "number" && Number.isFinite(value) ? Math.max(0, Math.trunc(value)) : 0;
}

export function getDeckOrigin(deckId: number): Origin {
  return { deck: deckId };
}
