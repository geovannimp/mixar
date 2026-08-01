/** MessagePack wire codec matching `crates/library-api` (named maps + Zod codecs). */

import { decode, encode } from "@msgpack/msgpack";
import { z } from "zod";

export const KindSchema = z.enum([
  "analyze_track",
  "track_analyzed",
  "refresh_track",
  "track_updated",
  "save_hot_cue",
  "delete_hot_cue",
  "save_loop",
  "delete_loop",
  "hot_cues_changed",
  "loops_changed",
  "error",
  "notice",
]);
export type Kind = z.infer<typeof KindSchema>;

export const OriginSchema = z.union([z.literal("library"), z.object({ track: z.string() })]);
export type Origin = z.infer<typeof OriginSchema>;

export const TrackSummarySchema = z.object({
  id: z.string(),
  display_name: z.string(),
  artist: z.string().nullable().optional(),
  title: z.string().nullable().optional(),
  album: z.string().nullable().optional(),
  genre: z.string().nullable().optional(),
  bpm: z.number().nullable().optional(),
  key: z.string().nullable().optional(),
  duration_ms: z.number().int().nullable().optional(),
  path: z.string(),
});
export type WireTrackSummary = z.infer<typeof TrackSummarySchema>;

export const HotCueSchema = z.object({
  slot: z.number().int().nonnegative(),
  position_ms: z.number().int(),
  loop_length_beats: z.number().int().nullable().optional(),
  color: z.string().nullable().optional(),
  label: z.string().nullable().optional(),
});
export type WireHotCue = z.infer<typeof HotCueSchema>;

export const SavedLoopSchema = z.object({
  slot: z.number().int().nonnegative(),
  in_ms: z.number().int(),
  out_ms: z.number().int(),
  label: z.string().nullable().optional(),
  color: z.string().nullable().optional(),
});
export type WireSavedLoop = z.infer<typeof SavedLoopSchema>;

export const CmdBodySchema = z.discriminatedUnion("type", [
  z.object({ type: z.literal("empty") }),
  z.object({
    type: z.literal("analyze_track"),
    track_id: z.string(),
    force: z.boolean(),
  }),
  z.object({
    type: z.literal("refresh_track"),
    track_id: z.string(),
  }),
  z.object({
    type: z.literal("save_hot_cue"),
    track_id: z.string(),
    slot: z.number().int().nonnegative(),
    position_ms: z.number().int(),
    loop_length_beats: z.number().int().nullable().optional(),
    color: z.string().nullable().optional(),
    label: z.string().nullable().optional(),
  }),
  z.object({
    type: z.literal("delete_hot_cue"),
    track_id: z.string(),
    slot: z.number().int().nonnegative(),
  }),
  z.object({
    type: z.literal("save_loop"),
    track_id: z.string(),
    slot: z.number().int().nonnegative(),
    in_ms: z.number().int(),
    out_ms: z.number().int(),
    label: z.string().nullable().optional(),
    color: z.string().nullable().optional(),
  }),
  z.object({
    type: z.literal("delete_loop"),
    track_id: z.string(),
    slot: z.number().int().nonnegative(),
  }),
]);
export type CmdBody = z.infer<typeof CmdBodySchema>;

export const EvtBodySchema = z.discriminatedUnion("type", [
  z.object({ type: z.literal("empty") }),
  z.object({ type: z.literal("track_analyzed"), track: TrackSummarySchema }),
  z.object({ type: z.literal("track_updated"), track: TrackSummarySchema }),
  z.object({
    type: z.literal("hot_cues_changed"),
    track_id: z.string(),
    hot_cues: z.array(HotCueSchema),
  }),
  z.object({
    type: z.literal("loops_changed"),
    track_id: z.string(),
    loops: z.array(SavedLoopSchema),
  }),
  z.object({
    type: z.literal("error"),
    message: z.string(),
    track_id: z.string().nullable().optional(),
  }),
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

export function encodeWire(message: WireMessage): Uint8Array {
  return asUint8Array(WireMessageCodec.encode(message));
}

export function decodeWire(bytes: Uint8Array): WireMessage {
  return WireMessageCodec.decode(bytes);
}

export function encodeCmdBody(body: CmdBody): Uint8Array {
  return asUint8Array(CmdBodyCodec.encode(body));
}

export function decodeCmdBody(bytes: Uint8Array): CmdBody {
  return CmdBodyCodec.decode(bytes);
}

export function encodeEvtBody(body: EvtBody): Uint8Array {
  return asUint8Array(EvtBodyCodec.encode(body));
}

export function decodeEvtBody(bytes: Uint8Array): EvtBody {
  return EvtBodyCodec.decode(bytes);
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

export type CmdKind =
  | "analyze_track"
  | "refresh_track"
  | "save_hot_cue"
  | "delete_hot_cue"
  | "save_loop"
  | "delete_loop";

/** Client-side evt filter; omit a field (or pass null) to match any. */
export type SubscribeFilter = {
  origin?: Origin | null;
  kind?: Kind | readonly Kind[] | null;
};

export function originsEqual(a: Origin, b: Origin): boolean {
  if (a === "library" || b === "library") {
    return a === b;
  }
  return a.track === b.track;
}

export function matchesSubscribeFilter(
  message: WireMessage,
  filter?: SubscribeFilter | null,
): boolean {
  if (!filter) {
    return true;
  }
  if (filter.origin != null && !originsEqual(message.origin, filter.origin)) {
    return false;
  }
  if (filter.kind != null) {
    const kinds = typeof filter.kind === "string" ? [filter.kind] : filter.kind;
    if (!kinds.includes(message.kind)) {
      return false;
    }
  }
  return true;
}

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

export function toTrackSummary(track: WireTrackSummary) {
  return {
    id: track.id,
    display_name: track.display_name,
    artist: track.artist ?? null,
    title: track.title ?? null,
    album: track.album ?? null,
    genre: track.genre ?? null,
    bpm: track.bpm ?? null,
    key: track.key ?? null,
    duration_ms: track.duration_ms ?? null,
    path: track.path,
  };
}
