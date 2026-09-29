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
}

/** One connector curve and the flow animation that drives it. */
interface LaneEntry {
  path: SVGPathElement;
  animation?: AnimationPlaybackControls;
}

/**
 * Animates the stem connector curves so dashes travel from the disc out to the
 * four lanes, reading as signal flowing through each stem. Muting a stem dims
 * its curve and pauses the flow, mirroring the waveform lane it feeds. Falls
 * back to a static solid connector when the visitor prefers reduced motion.
 */
export function initStemConnectorFlow(): StemConnectorFlow | undefined {
  const connector = document.querySelector<SVGSVGElement>("[data-stem-connector]");
  if (!connector) return undefined;

  const paths = Array.from(connector.querySelectorAll<SVGPathElement>("path[data-stem]"));
  if (paths.length === 0) return undefined;

  const reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  const lanes = new Map<number, LaneEntry>();

  for (const path of paths) {
    const entry: LaneEntry = { path };
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
    }
    lanes.set(Number(path.dataset.stem), entry);
  }

  return {
    setAudible(index, audible) {
      const lane = lanes.get(index);
      if (!lane) return;
      lane.path.style.opacity = audible ? "1" : String(MUTED_OPACITY);
      if (!lane.animation) return;
      if (audible) {
        lane.animation.play();
      } else {
        lane.animation.pause();
      }
    },
  };
}
