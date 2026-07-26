/** Postcard wire codec matching `crates/engine-api` (serde + postcard layout). */

export type Origin = "engine" | "mixer" | { deck: number };

export type Kind =
  | "play"
  | "pause"
  | "seek"
  | "set_volume"
  | "set_eq"
  | "set_speed"
  | "set_crossfader"
  | "set_cue_mix"
  | "set_master_cue"
  | "updated"
  | "position"
  | "levels"
  | "status"
  | "error"
  | "notice";

const KINDS: Kind[] = [
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
];

export interface DeckEq {
  low: number;
  mid: number;
  high: number;
}

export interface DeckSnapshot {
  id: number;
  playing: boolean;
  volume: number;
  speed: number;
  eq: DeckEq;
  position_secs: number | null;
  duration_secs: number | null;
}

export interface EngineStatusPayload {
  running: boolean;
  sample_rate: number;
  crossfader: number;
  cue_mix: number;
  master_cue: boolean;
  decks: DeckSnapshot[];
}

export type CmdBody =
  | { type: "empty" }
  | { type: "seek"; position_secs: number }
  | { type: "set_volume"; volume: number }
  | { type: "set_eq"; low: number; mid: number; high: number }
  | { type: "set_speed"; speed: number }
  | { type: "set_crossfader"; position: number }
  | { type: "set_cue_mix"; mix: number }
  | { type: "set_master_cue"; enabled: boolean };

export type EvtBody =
  | { type: "empty" }
  | {
      type: "deck_updated";
      id: number;
      playing: boolean;
      volume: number;
      speed: number;
      eq: DeckEq;
      position_secs: number | null;
      duration_secs: number | null;
    }
  | { type: "position"; position_secs: number }
  | {
      type: "levels";
      peak_l: number;
      peak_r: number;
      peak_hold_l: number;
      peak_hold_r: number;
    }
  | { type: "engine_status"; status: EngineStatusPayload }
  | { type: "error"; message: string }
  | { type: "notice"; message: string };

export interface WireMessage {
  origin: Origin;
  kind: Kind;
  revision: number;
  body: Uint8Array;
}

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
  return { origin, kind, revision, body };
}

export function encodeWire(message: WireMessage): Uint8Array {
  const out: number[] = [];
  writeOrigin(message.origin, out);
  writeKind(message.kind, out);
  writeVarintU64(message.revision, out);
  writeBytes(message.body, out);
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
  return body;
}

export function encodeCmdBody(body: CmdBody): Uint8Array {
  const out: number[] = [];
  switch (body.type) {
    case "empty":
      writeEnumTag(0, out);
      break;
    case "seek":
      writeEnumTag(1, out);
      writeF64Le(body.position_secs, out);
      break;
    case "set_volume":
      writeEnumTag(2, out);
      writeF32Le(body.volume, out);
      break;
    case "set_eq":
      writeEnumTag(3, out);
      writeF32Le(body.low, out);
      writeF32Le(body.mid, out);
      writeF32Le(body.high, out);
      break;
    case "set_speed":
      writeEnumTag(4, out);
      writeF32Le(body.speed, out);
      break;
    case "set_crossfader":
      writeEnumTag(5, out);
      writeF32Le(body.position, out);
      break;
    case "set_cue_mix":
      writeEnumTag(6, out);
      writeF32Le(body.mix, out);
      break;
    case "set_master_cue":
      writeEnumTag(7, out);
      writeBool(body.enabled, out);
      break;
    default: {
      const _exhaustive: never = body;
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
  return body;
}

export function encodeEvtBody(body: EvtBody): Uint8Array {
  const out: number[] = [];
  switch (body.type) {
    case "empty":
      writeEnumTag(0, out);
      break;
    case "deck_updated":
      writeEnumTag(1, out);
      writeVarintU16(body.id, out);
      writeBool(body.playing, out);
      writeF32Le(body.volume, out);
      writeF32Le(body.speed, out);
      writeDeckEq(body.eq, out);
      writeOptionF64(body.position_secs, out);
      writeOptionF64(body.duration_secs, out);
      break;
    case "position":
      writeEnumTag(2, out);
      writeF64Le(body.position_secs, out);
      break;
    case "levels":
      writeEnumTag(3, out);
      writeF32Le(body.peak_l, out);
      writeF32Le(body.peak_r, out);
      writeF32Le(body.peak_hold_l, out);
      writeF32Le(body.peak_hold_r, out);
      break;
    case "engine_status":
      writeEnumTag(4, out);
      writeBool(body.status.running, out);
      writeVarintU32(body.status.sample_rate, out);
      writeF32Le(body.status.crossfader, out);
      writeF32Le(body.status.cue_mix, out);
      writeBool(body.status.master_cue, out);
      writeDeckSnapshotVec(body.status.decks, out);
      break;
    case "error":
      writeEnumTag(5, out);
      writeString(body.message, out);
      break;
    case "notice":
      writeEnumTag(6, out);
      writeString(body.message, out);
      break;
    default: {
      const _exhaustive: never = body;
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
