/** Postcard wire codec matching `crates/engine-api` (serde + postcard layout). */

import { z } from "zod";

export const KindSchema = z.enum([
  "play",
  "pause",
  "seek",
  "set_volume",
  "set_eq",
  "set_speed",
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

const KINDS: Kind[] = KindSchema.options;

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

type Cursor = { pos: number };

function readU8(view: DataView, cursor: Cursor): number {
  const value = view.getUint8(cursor.pos);
  cursor.pos += 1;
  return value;
}

function writeU8(value: number, out: number[]): void {
  out.push(value & 0xff);
}

function readVarint(view: DataView, cursor: Cursor, maxBytes: number): bigint {
  let value = 0n;
  for (let i = 0; i < maxBytes; i += 1) {
    const byte = BigInt(readU8(view, cursor));
    value |= (byte & 0x7fn) << BigInt(i * 7);
    if ((byte & 0x80n) === 0n) {
      return value;
    }
  }
  throw new Error("varint overflow");
}

function writeVarint(value: number | bigint, maxBytes: number, out: number[]): void {
  let current = BigInt(value);
  for (let i = 0; i < maxBytes; i += 1) {
    let byte = Number(current & 0x7fn);
    current >>= 7n;
    if (i < maxBytes - 1 && current !== 0n) {
      byte |= 0x80;
    }
    out.push(byte);
    if (current === 0n) {
      return;
    }
  }
  if (current !== 0n) {
    throw new Error("varint overflow");
  }
}

function readVarintU16(view: DataView, cursor: Cursor): number {
  return Number(readVarint(view, cursor, 3));
}

function writeVarintU16(value: number, out: number[]): void {
  writeVarint(value, 3, out);
}

function readVarintU32(view: DataView, cursor: Cursor): number {
  return Number(readVarint(view, cursor, 5));
}

function writeVarintU32(value: number, out: number[]): void {
  writeVarint(value, 5, out);
}

function readVarintU64(view: DataView, cursor: Cursor): bigint {
  return readVarint(view, cursor, 10);
}

function writeVarintU64(value: number | bigint, out: number[]): void {
  writeVarint(value, 10, out);
}

function readF32Le(view: DataView, cursor: Cursor): number {
  const value = view.getFloat32(cursor.pos, true);
  cursor.pos += 4;
  return value;
}

function writeF32Le(value: number, out: number[]): void {
  const buffer = new ArrayBuffer(4);
  new DataView(buffer).setFloat32(0, value, true);
  out.push(...new Uint8Array(buffer));
}

function readF64Le(view: DataView, cursor: Cursor): number {
  const value = view.getFloat64(cursor.pos, true);
  cursor.pos += 8;
  return value;
}

function writeF64Le(value: number, out: number[]): void {
  const buffer = new ArrayBuffer(8);
  new DataView(buffer).setFloat64(0, value, true);
  out.push(...new Uint8Array(buffer));
}

function readBool(view: DataView, cursor: Cursor): boolean {
  const value = readU8(view, cursor);
  if (value === 0) return false;
  if (value === 1) return true;
  throw new Error(`invalid bool byte: ${value}`);
}

function writeBool(value: boolean, out: number[]): void {
  writeU8(value ? 1 : 0, out);
}

function readString(view: DataView, cursor: Cursor): string {
  const length = Number(readVarintU64(view, cursor));
  const bytes = new Uint8Array(length);
  for (let i = 0; i < length; i += 1) {
    bytes[i] = readU8(view, cursor);
  }
  return new TextDecoder().decode(bytes);
}

function writeString(value: string, out: number[]): void {
  const bytes = new TextEncoder().encode(value);
  writeVarintU64(bytes.length, out);
  out.push(...bytes);
}

function readBytes(view: DataView, cursor: Cursor): Uint8Array {
  const length = Number(readVarintU64(view, cursor));
  const bytes = new Uint8Array(length);
  for (let i = 0; i < length; i += 1) {
    bytes[i] = readU8(view, cursor);
  }
  return bytes;
}

function writeBytes(value: Uint8Array, out: number[]): void {
  writeVarintU64(value.length, out);
  out.push(...value);
}

function readEnumTag(view: DataView, cursor: Cursor): number {
  return Number(readVarintU64(view, cursor));
}

function writeEnumTag(tag: number, out: number[]): void {
  writeVarintU64(tag, out);
}

function readOptionF64(view: DataView, cursor: Cursor): number | null {
  const tag = readEnumTag(view, cursor);
  if (tag === 0) return null;
  if (tag === 1) return readF64Le(view, cursor);
  throw new Error(`invalid Option tag: ${tag}`);
}

function writeOptionF64(value: number | null, out: number[]): void {
  if (value === null) {
    writeEnumTag(0, out);
    return;
  }
  writeEnumTag(1, out);
  writeF64Le(value, out);
}

function readDeckEq(view: DataView, cursor: Cursor): DeckEq {
  return {
    low: readF32Le(view, cursor),
    mid: readF32Le(view, cursor),
    high: readF32Le(view, cursor),
  };
}

function writeDeckEq(eq: DeckEq, out: number[]): void {
  writeF32Le(eq.low, out);
  writeF32Le(eq.mid, out);
  writeF32Le(eq.high, out);
}

function readDeckSnapshot(view: DataView, cursor: Cursor): DeckSnapshot {
  return {
    id: readVarintU16(view, cursor),
    playing: readBool(view, cursor),
    volume: readF32Le(view, cursor),
    speed: readF32Le(view, cursor),
    eq: readDeckEq(view, cursor),
    position_secs: readOptionF64(view, cursor),
    duration_secs: readOptionF64(view, cursor),
  };
}

function writeDeckSnapshot(snapshot: DeckSnapshot, out: number[]): void {
  writeVarintU16(snapshot.id, out);
  writeBool(snapshot.playing, out);
  writeF32Le(snapshot.volume, out);
  writeF32Le(snapshot.speed, out);
  writeDeckEq(snapshot.eq, out);
  writeOptionF64(snapshot.position_secs, out);
  writeOptionF64(snapshot.duration_secs, out);
}

function readDeckSnapshotVec(view: DataView, cursor: Cursor): DeckSnapshot[] {
  const length = Number(readVarintU64(view, cursor));
  const decks: DeckSnapshot[] = [];
  for (let i = 0; i < length; i += 1) {
    decks.push(readDeckSnapshot(view, cursor));
  }
  return decks;
}

function writeDeckSnapshotVec(decks: DeckSnapshot[], out: number[]): void {
  writeVarintU64(decks.length, out);
  for (const deck of decks) {
    writeDeckSnapshot(deck, out);
  }
}

function readOrigin(view: DataView, cursor: Cursor): Origin {
  switch (readEnumTag(view, cursor)) {
    case 0:
      return "engine";
    case 1:
      return "mixer";
    case 2:
      return { deck: readVarintU16(view, cursor) };
    default:
      throw new Error("unknown Origin variant");
  }
}

function writeOrigin(origin: Origin, out: number[]): void {
  if (origin === "engine") {
    writeEnumTag(0, out);
    return;
  }
  if (origin === "mixer") {
    writeEnumTag(1, out);
    return;
  }
  writeEnumTag(2, out);
  writeVarintU16(origin.deck, out);
}

function readKind(view: DataView, cursor: Cursor): Kind {
  const tag = readEnumTag(view, cursor);
  const kind = KINDS[tag];
  if (!kind) {
    throw new Error(`unknown Kind variant: ${tag}`);
  }
  return kind;
}

function writeKind(kind: Kind, out: number[]): void {
  const tag = KINDS.indexOf(kind);
  if (tag < 0) {
    throw new Error(`unknown Kind: ${kind}`);
  }
  writeEnumTag(tag, out);
}

export function decodeWire(bytes: Uint8Array): WireMessage {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const cursor: Cursor = { pos: 0 };
  const origin = readOrigin(view, cursor);
  const kind = readKind(view, cursor);
  const revision = Number(readVarintU64(view, cursor));
  const body = readBytes(view, cursor);
  if (cursor.pos !== bytes.length) {
    throw new Error(`trailing bytes: ${bytes.length - cursor.pos}`);
  }
  return WireMessageSchema.parse({ origin, kind, revision, body });
}

export function encodeWire(message: WireMessage): Uint8Array {
  const valid = WireMessageSchema.parse(message);
  const out: number[] = [];
  writeOrigin(valid.origin, out);
  writeKind(valid.kind, out);
  writeVarintU64(valid.revision, out);
  writeBytes(valid.body, out);
  return Uint8Array.from(out);
}

export function decodeCmdBody(bytes: Uint8Array): CmdBody {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const cursor: Cursor = { pos: 0 };
  const body = (() => {
    switch (readEnumTag(view, cursor)) {
      case 0:
        return { type: "empty" } as const;
      case 1:
        return { type: "seek", position_secs: readF64Le(view, cursor) } as const;
      case 2:
        return { type: "set_volume", volume: readF32Le(view, cursor) } as const;
      case 3:
        return {
          type: "set_eq",
          low: readF32Le(view, cursor),
          mid: readF32Le(view, cursor),
          high: readF32Le(view, cursor),
        } as const;
      case 4:
        return { type: "set_speed", speed: readF32Le(view, cursor) } as const;
      case 5:
        return { type: "set_crossfader", position: readF32Le(view, cursor) } as const;
      case 6:
        return { type: "set_cue_mix", mix: readF32Le(view, cursor) } as const;
      case 7:
        return { type: "set_master_cue", enabled: readBool(view, cursor) } as const;
      default:
        throw new Error("unknown CmdBody variant");
    }
  })();
  if (cursor.pos !== bytes.length) {
    throw new Error(`trailing cmd body bytes: ${bytes.length - cursor.pos}`);
  }
  return CmdBodySchema.parse(body);
}

export function encodeCmdBody(body: CmdBody): Uint8Array {
  const valid = CmdBodySchema.parse(body);
  const out: number[] = [];
  switch (valid.type) {
    case "empty":
      writeEnumTag(0, out);
      break;
    case "seek":
      writeEnumTag(1, out);
      writeF64Le(valid.position_secs, out);
      break;
    case "set_volume":
      writeEnumTag(2, out);
      writeF32Le(valid.volume, out);
      break;
    case "set_eq":
      writeEnumTag(3, out);
      writeF32Le(valid.low, out);
      writeF32Le(valid.mid, out);
      writeF32Le(valid.high, out);
      break;
    case "set_speed":
      writeEnumTag(4, out);
      writeF32Le(valid.speed, out);
      break;
    case "set_crossfader":
      writeEnumTag(5, out);
      writeF32Le(valid.position, out);
      break;
    case "set_cue_mix":
      writeEnumTag(6, out);
      writeF32Le(valid.mix, out);
      break;
    case "set_master_cue":
      writeEnumTag(7, out);
      writeBool(valid.enabled, out);
      break;
    default: {
      const _exhaustive: never = valid;
      throw new Error(`unknown CmdBody: ${(_exhaustive as CmdBody).type}`);
    }
  }
  return Uint8Array.from(out);
}

export function decodeEvtBody(bytes: Uint8Array): EvtBody {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const cursor: Cursor = { pos: 0 };
  const body = (() => {
    switch (readEnumTag(view, cursor)) {
      case 0:
        return { type: "empty" } as const;
      case 1:
        return {
          type: "deck_updated",
          id: readVarintU16(view, cursor),
          playing: readBool(view, cursor),
          volume: readF32Le(view, cursor),
          speed: readF32Le(view, cursor),
          eq: readDeckEq(view, cursor),
          position_secs: readOptionF64(view, cursor),
          duration_secs: readOptionF64(view, cursor),
        } as const;
      case 2:
        return { type: "position", position_secs: readF64Le(view, cursor) } as const;
      case 3:
        return {
          type: "levels",
          peak_l: readF32Le(view, cursor),
          peak_r: readF32Le(view, cursor),
          peak_hold_l: readF32Le(view, cursor),
          peak_hold_r: readF32Le(view, cursor),
        } as const;
      case 4:
        return {
          type: "engine_status",
          status: {
            running: readBool(view, cursor),
            sample_rate: readVarintU32(view, cursor),
            crossfader: readF32Le(view, cursor),
            cue_mix: readF32Le(view, cursor),
            master_cue: readBool(view, cursor),
            decks: readDeckSnapshotVec(view, cursor),
          },
        } as const;
      case 5:
        return { type: "error", message: readString(view, cursor) } as const;
      case 6:
        return { type: "notice", message: readString(view, cursor) } as const;
      default:
        throw new Error("unknown EvtBody variant");
    }
  })();
  if (cursor.pos !== bytes.length) {
    throw new Error(`trailing evt body bytes: ${bytes.length - cursor.pos}`);
  }
  return EvtBodySchema.parse(body);
}

export function encodeEvtBody(body: EvtBody): Uint8Array {
  const valid = EvtBodySchema.parse(body);
  const out: number[] = [];
  switch (valid.type) {
    case "empty":
      writeEnumTag(0, out);
      break;
    case "deck_updated":
      writeEnumTag(1, out);
      writeVarintU16(valid.id, out);
      writeBool(valid.playing, out);
      writeF32Le(valid.volume, out);
      writeF32Le(valid.speed, out);
      writeDeckEq(valid.eq, out);
      writeOptionF64(valid.position_secs, out);
      writeOptionF64(valid.duration_secs, out);
      break;
    case "position":
      writeEnumTag(2, out);
      writeF64Le(valid.position_secs, out);
      break;
    case "levels":
      writeEnumTag(3, out);
      writeF32Le(valid.peak_l, out);
      writeF32Le(valid.peak_r, out);
      writeF32Le(valid.peak_hold_l, out);
      writeF32Le(valid.peak_hold_r, out);
      break;
    case "engine_status":
      writeEnumTag(4, out);
      writeBool(valid.status.running, out);
      writeVarintU32(valid.status.sample_rate, out);
      writeF32Le(valid.status.crossfader, out);
      writeF32Le(valid.status.cue_mix, out);
      writeBool(valid.status.master_cue, out);
      writeDeckSnapshotVec(valid.status.decks, out);
      break;
    case "error":
      writeEnumTag(5, out);
      writeString(valid.message, out);
      break;
    case "notice":
      writeEnumTag(6, out);
      writeString(valid.message, out);
      break;
    default: {
      const _exhaustive: never = valid;
      throw new Error(`unknown EvtBody: ${(_exhaustive as EvtBody).type}`);
    }
  }
  return Uint8Array.from(out);
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

type DeckCmdKind = "play" | "pause" | "seek" | "set_volume" | "set_eq" | "set_speed";
type MixerCmdKind = "set_crossfader" | "set_cue_mix" | "set_master_cue";

export function encodeWireCmd(origin: Origin, kind: Kind, body: CmdBody, revision = 0): Uint8Array {
  return encodeWire({
    origin,
    kind,
    revision,
    body: encodeCmdBody(body),
  });
}

export function encodeDeckCmd(
  deckId: number,
  kind: DeckCmdKind,
  body: CmdBody = { type: "empty" },
): Uint8Array {
  return encodeWireCmd({ deck: deckId }, kind, body);
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
