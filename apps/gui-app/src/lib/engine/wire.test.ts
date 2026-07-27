import { describe, expect, it } from "vitest";
import goldenHex from "@/lib/engine/golden/play_deck1.hex?raw";
import {
  bytesToHex,
  cmdBodyForKind,
  decodeCmdBody,
  decodeWire,
  encodeCmdBody,
  encodeWire,
  hexToBytes,
  type WireMessage,
} from "@/lib/engine/wire";

describe("wire codec", () => {
  it("decodes play_deck1 golden bytes", () => {
    const message = decodeWire(hexToBytes(goldenHex));

    expect(message.origin).toEqual({ deck: 1 });
    expect(message.kind).toBe("play");
    expect(message.revision).toBe(0);
    expect(decodeCmdBody(message.body)).toEqual({ type: "empty" });
  });

  it("encodes Play Deck(1) Empty revision 0 to golden hex", () => {
    const body = encodeCmdBody({ type: "empty" });
    const message: WireMessage = {
      origin: { deck: 1 },
      kind: "play",
      revision: 0,
      body,
    };

    expect(bytesToHex(encodeWire(message))).toBe(goldenHex.trim());
  });

  it("cmdBodyForKind uses empty when fields are omitted", () => {
    expect(cmdBodyForKind("play")).toEqual({ type: "empty" });
    expect(cmdBodyForKind("set_crossfader", { position: 0.5 })).toEqual({
      type: "set_crossfader",
      position: 0.5,
    });
  });
});
