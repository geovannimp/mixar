/** MessagePack wire codec matching `crates/engine-api` (named maps + Zod codecs). */

import { decode, encode } from "@msgpack/msgpack";
import { z } from "zod";

export const KindSchema = z.enum([
  "play",
  "pause",
  "seek",
  "set_volume",
  "set_eq",
  "set_speed",
  "set_filter",
  "set_gain_trim",
  "set_headphone_cue",
  "set_crossfader",
  "set_cue_mix",
  "set_master_cue",
  "toggle_sync",
  "set_master_deck",
  "unload",
  "set_cue_point",
  "begin_cue_hold",
  "end_cue_hold",
  "set_quantize",
  "set_auto_loop",
  "loop_in",
  "loop_out",
  "exit_loop",
  "beat_jump",
  "set_pad_mode",
  "begin_loop_roll",
  "end_loop_roll",
  "trigger_hot_cue",
  "recall_saved_loop",
  "trigger_sampler",
  "end_sampler",
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
  "updated",
  "position",
  "levels",
  "status",
  "error",
  "notice",
]);
export type Kind = z.infer<typeof KindSchema>;

export const OriginSchema = z.union([
  z.literal("engine"),
  z.literal("mixer"),
  z.object({ deck: z.number().int().nonnegative() }),
]);
export type Origin = z.infer<typeof OriginSchema>;

export const DeckEqSchema = z.object({
  low: z.number(),
  mid: z.number(),
  high: z.number(),
});
export type DeckEq = z.infer<typeof DeckEqSchema>;

export const SyncModeSchema = z.enum(["off", "tempo", "beat"]);
export type SyncMode = z.infer<typeof SyncModeSchema>;

export const PadModeSchema = z.enum(["hot_cue", "loop_roll", "beat_jump", "sampler"]);
export type PadMode = z.infer<typeof PadModeSchema>;

export const LoopRegionSchema = z.object({
  in_secs: z.number(),
  out_secs: z.number(),
  active: z.boolean(),
});
export type LoopRegion = z.infer<typeof LoopRegionSchema>;

export const DeckHotCueSchema = z.object({
  slot: z.number().int().nonnegative(),
  position_secs: z.number(),
  loop_length_beats: z.number().int().nullable().optional(),
  color: z.string().nullable().optional(),
  label: z.string().nullable().optional(),
});
export type DeckHotCue = z.infer<typeof DeckHotCueSchema>;

export const DeckSavedLoopSchema = z.object({
  slot: z.number().int().nonnegative(),
  in_secs: z.number(),
  out_secs: z.number(),
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
  duration_secs: z.number().nullable(),
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
  volume: z.number(),
  speed: z.number(),
  eq: DeckEqSchema,
  filter_db: z.number(),
  gain_trim_db: z.number(),
  headphone_cue: z.boolean(),
  sync_mode: SyncModeSchema,
  cue_point_secs: z.number().nullable(),
  quantize: z.boolean(),
  active_loop: LoopRegionSchema.nullable(),
  pad_mode: PadModeSchema,
  position_secs: z.number().nullable(),
  duration_secs: z.number().nullable(),
  hot_cues: z.array(DeckHotCueSchema),
  saved_loops: z.array(DeckSavedLoopSchema),
  loudness_lufs: z.number().nullable(),
  auto_gain_db: z.number(),
  active_sampler_bank_id: z.string().nullable(),
});
export type DeckSnapshot = z.infer<typeof DeckSnapshotSchema>;

export const EngineStatusPayloadSchema = z.object({
  running: z.boolean(),
  sample_rate: z.number().int().nonnegative(),
  crossfader: z.number(),
  cue_mix: z.number(),
  master_cue: z.boolean(),
  master_deck: z.number().int().nonnegative(),
  decks: z.array(DeckSnapshotSchema),
  sampler: SamplerStatusSchema,
});
export type EngineStatusPayload = z.infer<typeof EngineStatusPayloadSchema>;

export const CmdBodySchema = z.discriminatedUnion("type", [
  z.object({ type: z.literal("empty") }),
  z.object({ type: z.literal("seek"), position_secs: z.number() }),
  z.object({ type: z.literal("set_volume"), volume: z.number() }),
  z.object({
    type: z.literal("set_eq"),
    low: z.number(),
    mid: z.number(),
    high: z.number(),
  }),
  z.object({ type: z.literal("set_speed"), speed: z.number() }),
  z.object({ type: z.literal("set_filter"), filter_db: z.number() }),
  z.object({ type: z.literal("set_gain_trim"), gain_db: z.number() }),
  z.object({ type: z.literal("set_headphone_cue"), enabled: z.boolean() }),
  z.object({ type: z.literal("set_crossfader"), position: z.number() }),
  z.object({ type: z.literal("set_cue_mix"), mix: z.number() }),
  z.object({ type: z.literal("set_master_cue"), enabled: z.boolean() }),
  z.object({ type: z.literal("toggle_sync"), beat_sync: z.boolean() }),
  z.object({ type: z.literal("set_quantize"), enabled: z.boolean() }),
  z.object({ type: z.literal("set_auto_loop"), beats: z.number().int().nonnegative() }),
  z.object({ type: z.literal("beat_jump"), beats: z.number().int() }),
  z.object({ type: z.literal("set_pad_mode"), mode: PadModeSchema }),
  z.object({
    type: z.literal("begin_loop_roll"),
    beats: z.number().int().positive(),
  }),
  z.object({ type: z.literal("trigger_hot_cue"), position_secs: z.number() }),
  z.object({
    type: z.literal("recall_saved_loop"),
    in_secs: z.number(),
    out_secs: z.number(),
  }),
  z.object({ type: z.literal("trigger_sampler"), slot: z.number().int().nonnegative() }),
  z.object({ type: z.literal("end_sampler"), slot: z.number().int().nonnegative() }),
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
    volume: z.number(),
    speed: z.number(),
    eq: DeckEqSchema,
    filter_db: z.number(),
    gain_trim_db: z.number(),
    headphone_cue: z.boolean(),
    sync_mode: SyncModeSchema,
    cue_point_secs: z.number().nullable(),
    quantize: z.boolean(),
    active_loop: LoopRegionSchema.nullable(),
    pad_mode: PadModeSchema,
    position_secs: z.number().nullable(),
    duration_secs: z.number().nullable(),
    hot_cues: z.array(DeckHotCueSchema),
    saved_loops: z.array(DeckSavedLoopSchema),
    loudness_lufs: z.number().nullable(),
    auto_gain_db: z.number(),
    active_sampler_bank_id: z.string().nullable(),
  }),
  z.object({ type: z.literal("position"), position_secs: z.number() }),
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
  | "seek"
  | "set_volume"
  | "set_eq"
  | "set_speed"
  | "set_filter"
  | "set_gain_trim"
  | "set_headphone_cue"
  | "set_crossfader"
  | "set_cue_mix"
  | "set_master_cue"
  | "toggle_sync"
  | "set_master_deck"
  | "unload"
  | "set_cue_point"
  | "begin_cue_hold"
  | "end_cue_hold"
  | "set_quantize"
  | "set_auto_loop"
  | "loop_in"
  | "loop_out"
  | "exit_loop"
  | "beat_jump"
  | "set_pad_mode"
  | "begin_loop_roll"
  | "end_loop_roll"
  | "trigger_hot_cue"
  | "recall_saved_loop"
  | "trigger_sampler"
  | "end_sampler"
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
  | "load_library_track";

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
