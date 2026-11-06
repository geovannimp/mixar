import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// `vi.hoisted` keeps the spies stable across the module graph below, so the
// assertions observe the same functions the component calls.
const { initStemConnectorFlow } = vi.hoisted(() => ({ initStemConnectorFlow: vi.fn() }));

// The real module imports `motion`; nothing here needs it, and the whole point
// of the wrapper is that this chunk loads late.
vi.mock("./stem-connector-flow", () => ({ initStemConnectorFlow }));

// Resolve the gates immediately: the tests are about replaying state, not about
// when the import is allowed to start.
vi.mock("./idle", () => ({
  whenIdle: () => Promise.resolve(),
  whenVisible: () => Promise.resolve(),
}));

import { createDeferredConnectorFlow } from "./deferred-stem-connector-flow";

type Flow = { setAudible: ReturnType<typeof vi.fn>; setPlaying: ReturnType<typeof vi.fn> };

/**
 * Wait until the deferred import has actually run its bind step. Retrying on the
 * observable call beats counting macrotasks, which is a guess at how many turns
 * the module loader needs.
 */
const waitForBind = () => vi.waitFor(() => expect(initStemConnectorFlow).toHaveBeenCalled());

/** For the case where the module binds to nothing: give it a turn, then assert. */
const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

/** Roots live in the document: a detached one is skipped on purpose. */
const roots: HTMLElement[] = [];
const attachedRoot = () => {
  const root = document.createElement("div");
  document.body.append(root);
  roots.push(root);
  return root;
};

beforeEach(() => {
  initStemConnectorFlow.mockReset();
});

afterEach(() => {
  for (const root of roots.splice(0)) root.remove();
});

/** A bound flow stub, wired into the mocked module. */
function bindFlow(): Flow {
  const flow: Flow = { setAudible: vi.fn(), setPlaying: vi.fn() };
  initStemConnectorFlow.mockReturnValue(flow);
  return flow;
}

describe("createDeferredConnectorFlow", () => {
  // A mute pressed in the first second must survive the chunk arriving later,
  // and the newest value per lane must win — replaying the first call would
  // leave the lane in the opposite state to the pads.
  it("replays the latest state per lane once the flow binds", async () => {
    const flow = bindFlow();

    const deferred = createDeferredConnectorFlow(attachedRoot());
    deferred.setAudible(2, false);
    deferred.setPlaying(true);
    deferred.setAudible(2, true);
    deferred.setAudible(0, false);
    await waitForBind();

    expect(flow.setPlaying).toHaveBeenCalledWith(true);
    // One call per lane, carrying the last value written, not the first.
    expect(flow.setAudible.mock.calls).toEqual([
      [2, true],
      [0, false],
    ]);
  });

  it("forwards straight through once the flow exists", async () => {
    const flow = bindFlow();

    const deferred = createDeferredConnectorFlow(attachedRoot());
    await waitForBind();
    deferred.setAudible(1, false);
    deferred.setPlaying(false);

    expect(flow.setAudible).toHaveBeenCalledExactlyOnceWith(1, false);
    expect(flow.setPlaying).toHaveBeenCalledExactlyOnceWith(false);
  });

  // Pages without a connector (or without a `data-stems-demo` root) get an
  // undefined flow. The deferred one must bind to the no-op and keep accepting
  // writes without throwing.
  it("binds a no-op when the page has no connector", async () => {
    initStemConnectorFlow.mockReturnValue(undefined);

    const deferred = createDeferredConnectorFlow(attachedRoot());
    deferred.setPlaying(true);
    await settle();

    expect(initStemConnectorFlow).toHaveBeenCalledTimes(1);
    expect(() => deferred.setAudible(0, true)).not.toThrow();
  });

  // The router can swap the document while the chunk is in flight; binding then
  // would animate a detached tree.
  it("does not bind a root the router already removed", async () => {
    bindFlow();

    const root = attachedRoot();
    const deferred = createDeferredConnectorFlow(root);
    root.remove();
    await settle();
    await settle();

    expect(initStemConnectorFlow).not.toHaveBeenCalled();
    expect(() => deferred.setPlaying(true)).not.toThrow();
  });

  // A chunk that fails to load must degrade to the same no-op instead of
  // raising an unhandled rejection and buffering state forever.
  it("falls back to a no-op when the chunk fails", async () => {
    initStemConnectorFlow.mockImplementation(() => {
      throw new Error("chunk failed");
    });

    const deferred = createDeferredConnectorFlow(attachedRoot());
    deferred.setAudible(0, true);
    await settle();
    await settle();

    expect(initStemConnectorFlow).toHaveBeenCalledTimes(1);
    expect(() => deferred.setPlaying(true)).not.toThrow();
  });
});
