import { beforeEach, describe, expect, it, vi } from "vitest";

const { initHeroMobileTilt } = vi.hoisted(() => ({ initHeroMobileTilt: vi.fn() }));

// Both gates are stubbed to resolve immediately; what these tests exercise is
// ordering — whether the chunk is requested at all, and who disposes what.
vi.mock("./idle", () => ({
  whenIdle: () => Promise.resolve(),
  whenVisible: () => Promise.resolve(),
}));

import { createDeferredHeroTilt } from "./deferred-hero-tilt";

/** Let the stubbed gates and the loader settle. */
const settle = async () => {
  for (let tick = 0; tick < 3; tick++) await new Promise((resolve) => setTimeout(resolve, 0));
};

beforeEach(() => {
  initHeroMobileTilt.mockReset();
});

describe("createDeferredHeroTilt", () => {
  it("loads the module and binds the tilt once both gates pass", async () => {
    const disposer = vi.fn();
    initHeroMobileTilt.mockReturnValue(disposer);
    const load = vi.fn(async () => ({ initHeroMobileTilt }));

    createDeferredHeroTilt(document.createElement("div"), load);
    await settle();

    expect(load).toHaveBeenCalledTimes(1);
    expect(initHeroMobileTilt).toHaveBeenCalledTimes(1);
  });

  // Disposing before the gates resolve must skip the chunk entirely — that
  // chunk is the motion library this indirection exists to defer.
  it("never requests the chunk when disposed before the gates resolve", async () => {
    const load = vi.fn(async () => ({ initHeroMobileTilt }));

    const dispose = createDeferredHeroTilt(document.createElement("div"), load);
    dispose();
    await settle();

    expect(load).not.toHaveBeenCalled();
    expect(initHeroMobileTilt).not.toHaveBeenCalled();
  });

  it("releases a tilt it already started", async () => {
    const disposer = vi.fn();
    initHeroMobileTilt.mockReturnValue(disposer);
    const load = vi.fn(async () => ({ initHeroMobileTilt }));

    const dispose = createDeferredHeroTilt(document.createElement("div"), load);
    await settle();
    dispose();
    dispose();

    // Disposing twice must not run the tilt's own teardown twice.
    expect(disposer).toHaveBeenCalledTimes(1);
  });

  // A stale page after a deploy can 404 the chunk; that must not surface as an
  // unhandled rejection.
  it("swallows a failing chunk", async () => {
    const load = vi.fn(async () => {
      throw new Error("chunk failed");
    });

    const dispose = createDeferredHeroTilt(document.createElement("div"), load);
    await settle();

    expect(load).toHaveBeenCalledTimes(1);
    expect(() => dispose()).not.toThrow();
  });
});
