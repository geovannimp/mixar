import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// `idle` reads `requestIdleCallback` once, at module scope, so each test has to
// load a fresh copy after the stub is in place — hence the dynamic import and
// the module reset below.
type Idle = typeof import("./idle");
let whenIdle: Idle["whenIdle"];
let whenVisible: Idle["whenVisible"];

type IdleHandle = { timeout?: number };

/** Controllable `requestIdleCallback`, so the idle path is not a timing race. */
let idleRuns: Array<() => void> = [];

const installIdle = () => {
  Object.defineProperty(globalThis, "requestIdleCallback", {
    configurable: true,
    value: (run: () => void, _options?: IdleHandle) => {
      idleRuns.push(run);
      return idleRuns.length;
    },
  });
};

const removeIdle = () => {
  Reflect.deleteProperty(globalThis, "requestIdleCallback");
};

/** A document stub: `whenIdle` only reads readyState/defaultView and events. */
function fakeDocument(readyState: DocumentReadyState, withWindow = true) {
  const target = new EventTarget();
  const doc = Object.assign(target, {
    readyState,
    defaultView: withWindow ? (target as unknown as Window) : null,
  });
  return doc as unknown as Document;
}

beforeEach(async () => {
  idleRuns = [];
  installIdle();
  vi.resetModules();
  ({ whenIdle, whenVisible } = await import("./idle"));
});

afterEach(() => {
  removeIdle();
  vi.useRealTimers();
});

describe("whenIdle", () => {
  it("waits for an idle callback when the document is already complete", async () => {
    const doc = fakeDocument("complete");
    let resolved = false;
    const done = whenIdle(doc).then(() => {
      resolved = true;
    });

    await Promise.resolve();
    expect(resolved).toBe(false);
    expect(idleRuns).toHaveLength(1);

    idleRuns[0]?.();
    await done;
    expect(resolved).toBe(true);
  });

  it("resolves on the first intent event and detaches the others", async () => {
    const doc = fakeDocument("complete");
    const remove = vi.spyOn(doc, "removeEventListener");
    const done = whenIdle(doc);

    doc.dispatchEvent(new Event("pointerdown"));
    await done;

    // Every intent listener is torn down, not just the one that fired.
    expect(remove.mock.calls.map(([type]) => type).sort()).toEqual([
      "keydown",
      "pointerdown",
      "scroll",
    ]);
  });

  it("waits for the load event before scheduling idle", async () => {
    const doc = fakeDocument("loading");
    const done = whenIdle(doc);

    await Promise.resolve();
    expect(idleRuns).toHaveLength(0);

    (doc.defaultView as unknown as EventTarget).dispatchEvent(new Event("load"));
    expect(idleRuns).toHaveLength(1);
    idleRuns[0]?.();
    await done;
  });

  it("falls back to a macrotask without requestIdleCallback", async () => {
    // The engine has no idle API at all: reload the module without the stub so
    // it takes the setTimeout branch it would take on old Safari.
    removeIdle();
    vi.resetModules();
    whenIdle = (await import("./idle")).whenIdle;
    vi.useFakeTimers();
    const doc = fakeDocument("complete");
    let resolved = false;
    const done = whenIdle(doc).then(() => {
      resolved = true;
    });

    await Promise.resolve();
    expect(resolved).toBe(false);
    vi.runAllTimers();
    await done;
    expect(resolved).toBe(true);
  });
});

describe("whenVisible", () => {
  /** Minimal IntersectionObserver: the test decides when the entry fires. */
  let callbacks: Array<(entries: Array<{ isIntersecting: boolean }>) => void> = [];
  let disconnect: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    callbacks = [];
    disconnect = vi.fn();
    Object.defineProperty(globalThis, "IntersectionObserver", {
      configurable: true,
      value: class {
        constructor(callback: (entries: Array<{ isIntersecting: boolean }>) => void) {
          callbacks.push(callback);
        }
        observe() { }
        disconnect = disconnect;
      },
    });
  });

  afterEach(() => {
    Reflect.deleteProperty(globalThis, "IntersectionObserver");
  });

  it("resolves when the element comes into range and stops observing", async () => {
    const done = whenVisible(document.createElement("div"));
    let resolved = false;
    void done.then(() => {
      resolved = true;
    });

    callbacks[0]?.([{ isIntersecting: false }]);
    await Promise.resolve();
    expect(resolved).toBe(false);

    callbacks[0]?.([{ isIntersecting: true }]);
    await done;
    expect(resolved).toBe(true);
    expect(disconnect).toHaveBeenCalledTimes(1);
  });

  // Anything without IntersectionObserver (very old engines) must not stall —
  // the work is deferred, never dropped.
  it("resolves immediately without IntersectionObserver", async () => {
    Reflect.deleteProperty(globalThis, "IntersectionObserver");
    await expect(whenVisible(document.createElement("div"))).resolves.toBeUndefined();
  });
});
