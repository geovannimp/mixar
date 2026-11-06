import { whenIdle, whenVisible } from "./idle";

/** Stops the deferred bind, and any tilt it already started. */
export type DisposeTilt = () => void;

/**
 * Binds the hero screenshot's tilt once it is close to the viewport and the page
 * is idle, then returns a disposer.
 *
 * The tilt is decoration that pulls in the motion library, so the import waits
 * for both gates; `dispose` cancels the whole chain and, when the route swaps
 * before the gates resolve, the chunk is never requested at all.
 *
 * `load` is injected so a test can assert the import is skipped rather than
 * watching a network tab.
 */
export function createDeferredHeroTilt(
  tilt: Element,
  load: () => Promise<typeof import("./hero-mobile-tilt")> = () => import("./hero-mobile-tilt"),
): DisposeTilt {
  const gate = new AbortController();
  let disposeTilt: (() => void) | undefined;
  let cancelled = false;

  void whenVisible(tilt, "300px 0px", gate.signal)
    .then(() => whenIdle())
    .then(async () => {
      if (cancelled) return;
      const { initHeroMobileTilt } = await load();
      // A swap can also land while the chunk is in flight.
      if (cancelled) return;
      disposeTilt = initHeroMobileTilt();
    })
    // A stale page after a deploy can 404 the chunk; the tilt is decoration, so
    // drop it quietly instead of raising an unhandled rejection.
    .catch(() => {});

  return () => {
    cancelled = true;
    gate.abort();
    disposeTilt?.();
    disposeTilt = undefined;
  };
}
