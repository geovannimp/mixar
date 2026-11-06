import { beforeEach, describe, expect, it, vi } from "vitest";

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
 * Let the mocked gates and the dynamic import settle. The import resolves on a
 * macrotask, not a microtask, so this yields to the event loop rather than
 * counting promise ticks.
 */
const settle = async () => {
  for (let tick = 0; tick < 3; tick++) await new Promise((resolve) => setTimeout(resolve, 0));
};

beforeEach(() => {
  initStemConnectorFlow.mockReset();
});

describe("createDeferredConnectorFlow", () => {
  // A mute pressed in the first second must survive the chunk arriving later,
  // and the newest value per lane must win — replaying the first call would
  // leave the lane in the opposite state to the pads.
  it("replays the latest state per lane once the flow binds", async () => {
    const flow: Flow = { setAudible: vi.fn(), setPlaying: vi.fn() };
    initStemConnectorFlow.mockReturnValue(flow);

    const deferred = createDeferredConnectorFlow(document.createElement("div"));
    deferred.setAudible(2, false);
    deferred.setPlaying(true);
    deferred.setAudible(2, true);
    deferred.setAudible(0, false);
    await settle();

    expect(flow.setPlaying).toHaveBeenCalledWith(true);
    // One call per lane, carrying the last value written, not the first.
    expect(flow.setAudible.mock.calls).toEqual([
      [2, true],
      [0, false],
    ]);
  });

  it("forwards straight through once the flow exists", async () => {
    const flow: Flow = { setAudible: vi.fn(), setPlaying: vi.fn() };
    initStemConnectorFlow.mockReturnValue(flow);

    const deferred = createDeferredConnectorFlow(document.createElement("div"));
    await settle();
    deferred.setAudible(1, false);
    deferred.setPlaying(false);

    expect(flow.setAudible).toHaveBeenCalledExactlyOnceWith(1, false);
    expect(flow.setPlaying).toHaveBeenCalledExactlyOnceWith(false);
  });

  // Pages without a connector (or without a `data-stems-demo` root) get an
  // undefined flow; the deferred one must stop holding state for it.
  it("drops pending state when there is nothing to bind", async () => {
    initStemConnectorFlow.mockReturnValue(undefined);

    const deferred = createDeferredConnectorFlow(document.createElement("div"));
    deferred.setPlaying(true);
    await settle();

    expect(initStemConnectorFlow).toHaveBeenCalledTimes(1);
    // Still safe to call after the fact: no throw, nothing bound.
    expect(() => deferred.setAudible(0, true)).not.toThrow();
  });
});
