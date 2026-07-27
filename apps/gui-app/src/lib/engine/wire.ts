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

export const DeckSnapshotSchema = z.object({
  id: z.number().int().nonnegative(),
  playing: z.boolean(),
  volume: z.number(),
  speed: z.number(),
  eq: DeckEqSchema,
  filter_db: z.number(),
  gain_trim_db: z.number(),
  headphone_cue: z.boolean(),
  position_secs: z.number().nullable(),
  duration_secs: z.number().nullable(),
});
export type DeckSnapshot = z.infer<typeof DeckSnapshotSchema>;

export const EngineStatusPayloadSchema = z.object({
  running: z.boolean(),
  sample_rate: z.number().int().nonnegative(),
  crossfader: z.number(),
  cue_mix: z.number(),
  master_cue: z.boolean(),
  decks: z.array(DeckSnapshotSchema),
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
]);
export type CmdBody = z.infer<typeof CmdBodySchema>;

export const EvtBodySchema = z.discriminatedUnion("type", [
  z.object({ type: z.literal("empty") }),
  z.object({
    type: z.literal("deck_updated"),
    id: z.number().int().nonnegative(),
    playing: z.boolean(),
    volume: z.number(),
    speed: z.number(),
    eq: DeckEqSchema,
    filter_db: z.number(),
    gain_trim_db: z.number(),
    headphone_cue: z.boolean(),
    position_secs: z.number().nullable(),
    duration_secs: z.number().nullable(),
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
  body: z.instanceof(Uint8Array),
});
export type WireMessage = z.infer<typeof WireMessageSchema>;

const BytesSchema = z.instanceof(Uint8Array);

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

export type DeckCmdKind =
  | "play"
  | "pause"
  | "seek"
  | "set_volume"
  | "set_eq"
  | "set_speed"
  | "set_filter"
  | "set_gain_trim"
  | "set_headphone_cue";
export type MixerCmdKind = "set_crossfader" | "set_cue_mix" | "set_master_cue";

export function encodeWireCmd(origin: Origin, kind: Kind, body: CmdBody, revision = 0): Uint8Array {
  return encodeWire({
    origin,
    kind,
    revision,
    body: encodeCmdBody(body),
  });
}

export function getDeckOrigin(deckId: number): Origin {
  return { deck: deckId };
}

export function encodeDeckCmd(
  deckId: number,
  kind: DeckCmdKind,
  body: CmdBody = { type: "empty" },
): Uint8Array {
  return encodeWireCmd(getDeckOrigin(deckId), kind, body);
}

export function encodePlay(deckId: number): Uint8Array {
  return encodeDeckCmd(deckId, "play");
}

export function encodePause(deckId: number): Uint8Array {
  return encodeDeckCmd(deckId, "pause");
}

export function encodeSeek(deckId: number, positionSecs: number): Uint8Array {
  return encodeDeckCmd(deckId, "seek", { type: "seek", position_secs: positionSecs });
}

export function encodeSetVolume(deckId: number, volume: number): Uint8Array {
  return encodeDeckCmd(deckId, "set_volume", { type: "set_volume", volume });
}

export function encodeSetEq(deckId: number, low: number, mid: number, high: number): Uint8Array {
  return encodeDeckCmd(deckId, "set_eq", { type: "set_eq", low, mid, high });
}

export function encodeSetSpeed(deckId: number, speed: number): Uint8Array {
  return encodeDeckCmd(deckId, "set_speed", { type: "set_speed", speed });
}

export function encodeSetFilter(deckId: number, filterDb: number): Uint8Array {
  return encodeDeckCmd(deckId, "set_filter", { type: "set_filter", filter_db: filterDb });
}

export function encodeSetGainTrim(deckId: number, gainDb: number): Uint8Array {
  return encodeDeckCmd(deckId, "set_gain_trim", { type: "set_gain_trim", gain_db: gainDb });
}

export function encodeSetHeadphoneCue(deckId: number, enabled: boolean): Uint8Array {
  return encodeDeckCmd(deckId, "set_headphone_cue", {
    type: "set_headphone_cue",
    enabled,
  });
}

export function encodeMixerCmd(kind: MixerCmdKind, body: CmdBody): Uint8Array {
  return encodeWireCmd("mixer", kind, body);
}

export function encodeSetCrossfader(position: number): Uint8Array {
  return encodeMixerCmd("set_crossfader", { type: "set_crossfader", position });
}

export function encodeSetCueMix(mix: number): Uint8Array {
  return encodeMixerCmd("set_cue_mix", { type: "set_cue_mix", mix });
}

export function encodeSetMasterCue(enabled: boolean): Uint8Array {
  return encodeMixerCmd("set_master_cue", { type: "set_master_cue", enabled });
}
