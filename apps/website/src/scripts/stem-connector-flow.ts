import { animate, type AnimationPlaybackControls } from "motion";

/** Dash geometry, in the connector's viewBox units. */
const DASH = 6;
const GAP = 10;
const PERIOD = DASH + GAP;

/** Seconds for a dash to travel one period, so the flow speed stays steady. */
const CYCLE_SECONDS = 1.1;

/** Opacity for a muted lane's connector, matching the dimmed waveform lane. */
const MUTED_OPACITY = 0.25;

/** Per-stem handle so the demo can mute a lane's connector with its waveform. */
export interface StemConnectorFlow {
  setAudible(index: number, audible: boolean): void;
  /** Flow dashes only while the demo is actually playing audio. */
  setPlaying(playing: boolean): void;
}

/** One connector curve and the flow animation that drives it. */
interface LaneEntry {
  path: SVGPathElement;
  animation?: AnimationPlaybackControls;
  audible: boolean;
}

/**
 * Animates the stem connector curves so dashes travel from the disc out to the
 * four lanes, reading as signal flowing through each stem. Muting a stem dims
 * its curve and pauses the flow, mirroring the waveform lane it feeds. Flow
 * only runs while audio is playing. Falls back to a static solid connector when
 * the visitor prefers reduced motion.
 *
 * Scoped to `root` so a second stems demo on the page gets its own flow.
 */
export function initStemConnectorFlow(root: ParentNode = document): StemConnectorFlow | undefined {
  const connector = root.querySelector<SVGSVGElement>("[data-stem-connector]");
  if (!connector || connector.dataset.flowInit !== undefined) return undefined;
  connector.dataset.flowInit = "";

  const paths = Array.from(connector.querySelectorAll<SVGPathElement>("path[data-stem]"));
  if (paths.length === 0) return undefined;

  const disc = root.querySelector<HTMLElement>("[data-stem-disc]");
  const reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  const lanes = new Map<number, LaneEntry>();
  let onScreen = true;
  let playing = false;

  // Flow only while audio is playing, the connector is on screen, and the lane
  // is audible — otherwise the endless dash loop costs nothing.
  const sync = (lane: LaneEntry) => {
    if (!lane.animation) return;
    if (playing && onScreen && lane.audible) {
      lane.animation.play();
    } else {
      lane.animation.pause();
    }
  };

  const syncAll = () => {
    for (const lane of lanes.values()) sync(lane);
  };

  const syncDisc = () => {
    disc?.setAttribute("data-playing", String(playing && onScreen));
  };

  for (const path of paths) {
    const entry: LaneEntry = { path, audible: true };
    if (!reduceMotion) {
      path.style.transition = "opacity 0.2s ease";
      path.style.strokeDasharray = `${DASH} ${GAP}`;
      entry.animation = animate(
        path,
        // Decreasing the offset moves the dashes from the shared start (the disc)
        // toward each lane's end, and -PERIOD is one whole dash period, so the
        // loop restarts in phase with no visible jump.
        { strokeDashoffset: [0, -PERIOD] },
        { duration: CYCLE_SECONDS, ease: "linear", repeat: Infinity },
      );
      // Start paused until the demo actually plays audio.
      entry.animation.pause();
    }
    lanes.set(Number(path.dataset.stem), entry);
  }

  if (!reduceMotion) {
    const observer = new IntersectionObserver(([entry]) => {
      onScreen = entry?.isIntersecting ?? true;
      syncAll();
      syncDisc();
    });
    observer.observe(connector);
  }

  return {
    setAudible(index, audible) {
      const lane = lanes.get(index);
      if (!lane) return;
      lane.audible = audible;
      lane.path.style.opacity = audible ? "1" : String(MUTED_OPACITY);
      sync(lane);
    },
    setPlaying(next) {
      if (playing === next) return;
      playing = next;
      syncAll();
      syncDisc();
    },
  };
}
