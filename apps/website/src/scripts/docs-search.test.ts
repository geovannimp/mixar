// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";

import type { SearchIndex, SearchResult } from "../lib/docs-search";
import { initDocsSearch } from "./docs-search";

const result: SearchResult = {
  document: {
    title: "Performance pads",
    path: "/docs/users/performance-pads",
    category: "Users",
    topic: "Performance pads",
    description: "",
    text: "Play cues with the pads.",
  },
  score: 0,
  snippet: [{ text: "pads", match: true }],
};

class SearchWorker {
  private readonly listeners = new Map<string, ((event: MessageEvent) => void)[]>();

  addEventListener(type: string, listener: (event: MessageEvent) => void): void {
    const listeners = this.listeners.get(type) ?? [];
    listeners.push(listener);
    this.listeners.set(type, listeners);
  }

  postMessage(message: { type: string; id?: number }): void {
    if (message.type === "index") {
      queueMicrotask(() => this.emit({ type: "ready" }));
      return;
    }
    if (message.type === "search" && message.id !== undefined) {
      queueMicrotask(() => this.emit({ type: "results", id: message.id, results: [result] }));
    }
  }

  terminate(): void {}

  private emit(data: unknown): void {
    for (const listener of this.listeners.get("message") ?? []) {
      listener(new MessageEvent("message", { data }));
    }
  }
}

afterEach(() => {
  document.body.replaceChildren();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe("docs search UI", () => {
  it("retries the index fetch on input after the first fetch fails", async () => {
    document.body.innerHTML = `
      <button id="docs-search-open"></button>
      <dialog id="docs-search-dialog">
        <input id="docs-search-input" />
        <ul id="docs-search-results"></ul>
        <p id="docs-search-empty"></p>
        <p id="docs-search-status"></p>
        <button data-docs-search-scope="users" aria-pressed="true"></button>
        <button data-docs-search-scope="developers" aria-pressed="false"></button>
      </dialog>`;
    Object.defineProperty(HTMLDialogElement.prototype, "showModal", {
      configurable: true,
      value() {
        this.setAttribute("open", "");
      },
    });

    let fetchCount = 0;
    vi.stubGlobal("fetch", vi.fn(async () => {
      fetchCount++;
      if (fetchCount === 1) throw new Error("temporary network failure");
      return { ok: true, json: async (): Promise<SearchIndex> => ({ documents: [] }) };
    }));
    vi.stubGlobal("Worker", SearchWorker);
    vi.spyOn(console, "error").mockImplementation(() => {});

    initDocsSearch();
    document.getElementById("docs-search-open")?.click();
    await vi.waitFor(() =>
      expect(document.getElementById("docs-search-empty")?.textContent).toContain("unavailable"),
    );

    const input = document.getElementById("docs-search-input") as HTMLInputElement;
    input.value = "pads";
    input.dispatchEvent(new Event("input", { bubbles: true }));

    await vi.waitFor(() => {
      expect(fetchCount).toBe(2);
      expect(document.getElementById("docs-search-results")?.textContent).toContain("Performance pads");
    });
    expect(document.querySelector("#docs-search-results mark")?.textContent).toBe("pads");
  });
});
