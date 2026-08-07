import { describe, expect, it } from "vitest";
import { focusedLoadTargetFromRow } from "@/lib/library/focused-load";
import { libraryRowFromFile } from "@/lib/library-table";
import type { FsEntry, TrackSummary } from "@/types";

function entry(path: string): FsEntry {
  return { path, name: path.split("/").pop() ?? path };
}

function summary(id: string, path: string): TrackSummary {
  return {
    id,
    display_name: id,
    artist: null,
    title: null,
    album: null,
    genre: null,
    bpm: null,
    key: null,
    duration_ms: null,
    path,
  };
}

describe("focusedLoadTargetFromRow", () => {
  it("returns null for missing row", () => {
    expect(focusedLoadTargetFromRow(undefined)).toBeNull();
  });

  it("indexes the visible list, not the unfiltered source", () => {
    const unfiltered = [
      libraryRowFromFile(entry("/a.wav"), summary("a", "/a.wav")),
      libraryRowFromFile(entry("/b.wav"), summary("b", "/b.wav")),
      libraryRowFromFile(entry("/match.wav"), summary("m", "/match.wav")),
    ];
    // Filter dropped /a and /b — visible index 0 is /match, not /a.
    const visible = [unfiltered[2]!];
    expect(focusedLoadTargetFromRow(visible[0])).toEqual({ trackId: "m" });
    expect(focusedLoadTargetFromRow(unfiltered[0])).toEqual({ trackId: "a" });
  });

  it("prefers filesystem path when no library track", () => {
    expect(focusedLoadTargetFromRow(libraryRowFromFile(entry("/only.wav")))).toEqual({
      path: "/only.wav",
    });
  });
});
