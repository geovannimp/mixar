import type { StemConnectorFlow } from "./stem-connector-flow";
import { whenIdle, whenVisible } from "./idle";

/** Stands in for a connector that failed to load, so later writes are no-ops. */
const NO_FLOW: StemConnectorFlow = {
  setAudible: () => {},
  setPlaying: () => {},
};

/**
 * A `StemConnectorFlow` that exists before its module does.
 *
 * The connector animation is decorative, and the module behind it imports the
 * motion library (~22KB) — eager loading made both compete with the demo's own
 * paint. The demo can wire its pads immediately: the last state written per lane,
 * plus the play flag, is replayed onto the real flow once it binds, so a mute in
 * the first second still lands.
 *
 * Dynamic import on purpose: the specifier is fixed, but a static import would
 * put the module in the entry chunk graph, and the browser would then fetch the
 * motion library with the initial load — the exact thing being avoided.
 */
export function createDeferredConnectorFlow(root: ParentNode): StemConnectorFlow {
  let real: StemConnectorFlow | undefined;
  // Last-write-wins state, not an event log: replaying it cannot grow with the
  // visitor's clicks and cannot apply a stale value after a newer one.
  const audible = new Map<number, boolean>();
  let playing: boolean | undefined;

  // The connector belongs to the Stems section, which on a phone is several
  // screens down: nothing is fetched until that section is close to the
  // viewport, and then only once the page is idle.
  const anchor = root instanceof Element ? root : undefined;
  const gate = new AbortController();
  const near =
    anchor === undefined
      ? whenIdle()
      : whenVisible(anchor, "300px 0px", gate.signal).then(() => whenIdle());

  // A swap can land while the section is still off-screen; release the observer
  // and the idle listeners with the page they belong to.
  if (anchor !== undefined) {
    anchor.ownerDocument.addEventListener("astro:before-swap", () => gate.abort(), { once: true });
  }

  void near
    .then(async () => {
      const { initStemConnectorFlow } = await import("./stem-connector-flow");
      // The router can swap the document while this chunk is in flight; binding
      // then would animate a detached tree nobody sees.
      if (anchor !== undefined && !anchor.isConnected) {
        real = NO_FLOW;
        return;
      }
      real = initStemConnectorFlow(root) ?? NO_FLOW;
      if (playing !== undefined) real.setPlaying(playing);
      for (const [index, isAudible] of audible) real.setAudible(index, isAudible);
    })
    .catch(() => {
      // Decorative: a missing chunk or a throw inside the module must not
      // surface as an unhandled rejection, and the buffered state has to stop
      // growing, so bind the no-op instead of retrying forever.
      real = NO_FLOW;
    });

  return {
    setAudible: (index, isAudible) => {
      if (real !== undefined) {
        real.setAudible(index, isAudible);
        return;
      }
      audible.set(index, isAudible);
    },
    setPlaying: (isPlaying) => {
      if (real !== undefined) {
        real.setPlaying(isPlaying);
        return;
      }
      playing = isPlaying;
    },
  };
}
