/**
 * Runs work once the page has stopped being busy with the visitor's first paint,
 * or as soon as they show intent, whichever comes first.
 *
 * Everything decorative here — the analytics SDK, the motion library behind the
 * hero tilt and the stem connector — is worth zero milliseconds to the first
 * screen, and on a throttled phone each eager import delays exactly the paint
 * the visitor is waiting for.
 */
const IDLE_TIMEOUT = 2000;

/** Cheap-to-observe signals that the visitor is engaging, not just landing. */
const INTENT_EVENTS = ["pointerdown", "keydown", "scroll"] as const;

/** `requestIdleCallback` is still missing from older Safari. */
const idleCallback: typeof requestIdleCallback | undefined =
  typeof globalThis.requestIdleCallback === "function"
    ? globalThis.requestIdleCallback.bind(globalThis)
    : undefined;

function scheduleIdle(run: () => void): void {
  if (idleCallback !== undefined) {
    // The timeout is the safety net: a busy main thread must not starve the
    // work forever, or the analytics chunk would never load at all.
    idleCallback(run, { timeout: IDLE_TIMEOUT });
    return;
  }
  setTimeout(run, 0);
}

export function whenIdle(doc: Document = document): Promise<void> {
  // Plain executors rather than `Promise.withResolvers`: that landed in Chrome
  // 119 / Safari 17.4 / Firefox 121, and a synchronous throw here would abort
  // the caller's wiring (the stems demo, analytics) on an older browser.
  return new Promise((resolve) => {
    let settled = false;
    const done = () => {
      if (settled) return;
      settled = true;
      for (const type of INTENT_EVENTS) doc.removeEventListener(type, done, true);
      resolve();
    };

    for (const type of INTENT_EVENTS) {
      doc.addEventListener(type, done, { capture: true, once: true, passive: true });
    }

    if (doc.readyState === "complete" || doc.defaultView === null) {
      scheduleIdle(done);
      return;
    }
    // `load` is the point after which the main thread is ours to spend; the idle
    // callback then keeps the work out of the load event's own critical path.
    doc.defaultView.addEventListener("load", () => scheduleIdle(done), { once: true });
  });
}

/**
 * Resolves the first time `el` comes within `rootMargin` of the viewport.
 *
 * The hero screenshot and the stems connector are decorative and sit at or below
 * the fold, so their animation library does not need to exist until the visitor
 * can see them. `rootMargin` buys the import enough lead time that the animation
 * is ready by the time the element scrolls in.
 *
 * `signal` releases the observer when the caller's page goes away: a route swap
 * can happen while the element is still off-screen, and an un-abortable observer
 * would keep both itself and the deferred import alive. Aborting resolves rather
 * than rejects, so the caller's chain runs and its own checks decide what to do.
 */
export function whenVisible(
  el: Element,
  rootMargin = "300px 0px",
  signal?: AbortSignal,
): Promise<void> {
  if (typeof IntersectionObserver !== "function") return Promise.resolve();
  return new Promise((resolve) => {
    let done = false;
    const finish = () => {
      if (done) return;
      done = true;
      observer.disconnect();
      signal?.removeEventListener("abort", finish);
      resolve();
    };
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) finish();
      },
      { rootMargin },
    );
    if (signal?.aborted) {
      resolve();
      return;
    }
    signal?.addEventListener("abort", finish, { once: true });
    observer.observe(el);
  });
}
