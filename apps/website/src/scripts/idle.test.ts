import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// `idle` reads `requestIdleCallback` once, at module scope, so each test has to
// load a fresh copy after the stub is in place — hence the dynamic import and
// the module reset below.
type Idle = typeof import("./idle");
let whenIdle: Idle["whenIdle"];
let whenVisible: Idle["whenVisible"];

type IdleOptions = { timeout?: number };

/** Controllable `requestIdleCallback`, so the idle path is not a timing race. */
let idleCalls: Array<{ run: () => void; options?: IdleOptions }> = [];

const installIdle = () => {
  Object.defineProperty(globalThis, "requestIdleCallback", {
    configurable: true,
    value: (run: () => void, options?: IdleOptions) => {
      idleCalls.push({ run, options });
      return idleCalls.length;
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

/** Tracks whether a promise has settled, without awaiting it. */
function trackResolved(promise: Promise<void>) {
  const state = { resolved: false };
  void promise.then(() => {
    state.resolved = true;
  });
  return state;
}

const runIdle = (index = 0) => idleCalls[index]?.run();

/** Run the queued idle callback and wait for the gate to resolve. */
const settleIdle = async (index = 0) => {
  runIdle(index);
  await vi.waitFor(() => expect(idleCalls).toHaveLength(index + 1));
};

beforeEach(async () => {
  idleCalls = [];
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
    const state = trackResolved(whenIdle(doc));

    await Promise.resolve();
    expect(state.resolved).toBe(false);
    expect(idleCalls).toHaveLength(1);
    // The timeout is what keeps a busy main thread from starving the work.
    expect(idleCalls[0]?.options).toEqual({ timeout: 2000 });

    await settleIdle();
    expect(state.resolved).toBe(true);
  });

  it("schedules idle straight away for a document without a window", async () => {
    const doc = fakeDocument("loading", false);
    const state = trackResolved(whenIdle(doc));

    await Promise.resolve();
    expect(idleCalls).toHaveLength(1);

    await settleIdle();
    expect(state.resolved).toBe(true);
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
    const state = trackResolved(whenIdle(doc));

    await Promise.resolve();
    expect(idleCalls).toHaveLength(0);

    (doc.defaultView as unknown as EventTarget).dispatchEvent(new Event("load"));
    expect(idleCalls).toHaveLength(1);
    await settleIdle();
    expect(state.resolved).toBe(true);
  });

  it("falls back to a macrotask without requestIdleCallback", async () => {
    // The engine has no idle API at all: reload the module without the stub so
    // it takes the setTimeout branch it would take on old Safari.
    removeIdle();
    vi.resetModules();
    whenIdle = (await import("./idle")).whenIdle;
    vi.useFakeTimers();
    const doc = fakeDocument("complete");
    const state = trackResolved(whenIdle(doc));

    await Promise.resolve();
    expect(state.resolved).toBe(false);
    vi.runAllTimers();
    await vi.waitFor(() => expect(state.resolved).toBe(true));
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
        observe() {}
        disconnect = disconnect;
      },
    });
  });

  afterEach(() => {
    Reflect.deleteProperty(globalThis, "IntersectionObserver");
  });

  it("resolves when the element comes into range and stops observing", async () => {
    const state = trackResolved(whenVisible(document.createElement("div")));

    callbacks[0]?.([{ isIntersecting: false }]);
    await Promise.resolve();
    expect(state.resolved).toBe(false);

    callbacks[0]?.([{ isIntersecting: true }]);
    await vi.waitFor(() => expect(state.resolved).toBe(true));
    expect(disconnect).toHaveBeenCalledTimes(1);
  });

  // A route swap while the element is still off-screen has to release the
  // observer; resolving (not rejecting) lets the caller's chain continue.
  it("stops observing when the caller aborts", async () => {
    const gate = new AbortController();
    const state = trackResolved(whenVisible(document.createElement("div"), "0px", gate.signal));

    gate.abort();
    await vi.waitFor(() => expect(state.resolved).toBe(true));
    expect(disconnect).toHaveBeenCalledTimes(1);
  });

  it("resolves straight away for an already-aborted signal", async () => {
    const gate = new AbortController();
    gate.abort();
    await expect(
      whenVisible(document.createElement("div"), "0px", gate.signal),
    ).resolves.toBeUndefined();
    expect(disconnect).not.toHaveBeenCalled();
  });

  // Anything without IntersectionObserver (very old engines) must not stall —
  // the work is deferred, never dropped.
  it("resolves immediately without IntersectionObserver", async () => {
    Reflect.deleteProperty(globalThis, "IntersectionObserver");
    await expect(whenVisible(document.createElement("div"))).resolves.toBeUndefined();
  });
});
