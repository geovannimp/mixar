import {
  formatDeckTime,
  StemDemoPlayer,
  STEM_UNAVAILABLE_MESSAGE,
  type StemDemoSnapshot,
} from "./stem-demo-player";

type WaveColors = { wave: string; playhead: string; bg: string };

/** Paint peaks into an offscreen layer (rebuilt only when size/peaks/colors change). */
function paintPeaksLayer(
  layer: HTMLCanvasElement,
  peaks: Float32Array | null,
  colors: WaveColors,
): void {
  const ctx = layer.getContext("2d");
  if (!ctx) return;
  const { width, height } = layer;
  ctx.clearRect(0, 0, width, height);
  ctx.fillStyle = colors.bg;
  ctx.fillRect(0, 0, width, height);

  const mid = height / 2;
  if (!peaks || peaks.length === 0) {
    ctx.strokeStyle = "rgba(255,255,255,0.12)";
    ctx.beginPath();
    ctx.moveTo(0, mid);
    ctx.lineTo(width, mid);
    ctx.stroke();
    return;
  }

  ctx.fillStyle = colors.wave;
  ctx.globalAlpha = 0.92;
  ctx.beginPath();
  ctx.moveTo(0, mid);
  for (let i = 0; i < peaks.length; i++) {
    const x = (i / (peaks.length - 1 || 1)) * width;
    const amp = (peaks[i] ?? 0) * (height * 0.42);
    ctx.lineTo(x, mid - amp);
  }
  for (let i = peaks.length - 1; i >= 0; i--) {
    const x = (i / (peaks.length - 1 || 1)) * width;
    const amp = (peaks[i] ?? 0) * (height * 0.42);
    ctx.lineTo(x, mid + amp);
  }
  ctx.closePath();
  ctx.fill();
  ctx.globalAlpha = 1;
}

/** Compose cached peaks + playhead onto the visible canvas. */
function drawStemWaveform(
  canvas: HTMLCanvasElement,
  peaksLayer: HTMLCanvasElement,
  position: number,
  duration: number,
  playhead: string,
): void {
  if (canvas.width !== peaksLayer.width || canvas.height !== peaksLayer.height) {
    canvas.width = peaksLayer.width;
    canvas.height = peaksLayer.height;
  }
  const ctx = canvas.getContext("2d");
  if (!ctx) return;

  ctx.clearRect(0, 0, canvas.width, canvas.height);
  ctx.drawImage(peaksLayer, 0, 0);

  if (duration > 0) {
    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    const x = (position / duration) * canvas.width;
    ctx.strokeStyle = playhead;
    ctx.lineWidth = Math.max(1, dpr);
    ctx.beginPath();
    ctx.moveTo(x, 0);
    ctx.lineTo(x, canvas.height);
    ctx.stroke();
  }
}

function readThemeColor(el: Element, name: string, fallback: string): string {
  const value = getComputedStyle(el).getPropertyValue(name).trim();
  return value || fallback;
}

function readPalette(root: Element): WaveColors {
  return {
    wave: readThemeColor(root, "--accent", "#2dd4bf"),
    playhead: "#f4f4f5",
    bg: "#000000",
  };
}

export interface StemDemoPlayerHandle {
  setAudible: (audible: boolean[]) => void;
  dispose: () => void;
}

export interface StemDemoPlayerUiOptions {
  /** Fired when transport enters/leaves playing (not on every tick). */
  onPlayingChange?: (playing: boolean) => void;
}

const handlesByRoot = new WeakMap<HTMLElement, StemDemoPlayerHandle>();
const liveHandles = new Set<StemDemoPlayerHandle>();
let teardownWired = false;

/** Tear down every live stem demo player (pagehide / Astro swap). */
function disposeAllStemDemoPlayers(): void {
  for (const handle of [...liveHandles]) handle.dispose();
}

function ensureGlobalTeardown(): void {
  if (teardownWired) return;
  teardownWired = true;
  // Persistent listeners: dispose is idempotent and liveHandles is empty after teardown,
  // so repeated pagehide / before-swap events are cheap and cover soft navigations.
  window.addEventListener("pagehide", disposeAllStemDemoPlayers);
  document.addEventListener("astro:before-swap", disposeAllStemDemoPlayers);
}

/**
 * Bind the deck-style transport panel inside a stems demo root.
 * Load is deferred until the first Play so the marketing page stays light.
 */
export function initStemDemoPlayer(
  root: HTMLElement,
  opts: StemDemoPlayerUiOptions = {},
): StemDemoPlayerHandle {
  handlesByRoot.get(root)?.dispose();
  ensureGlobalTeardown();

  const playBtn = root.querySelector<HTMLButtonElement>("[data-stem-play]");
  const titleEl = root.querySelector<HTMLElement>("[data-stem-title]");
  const remainingEl = root.querySelector<HTMLElement>("[data-stem-remaining]");
  const durationEl = root.querySelector<HTMLElement>("[data-stem-duration]");
  const statusEl = root.querySelector<HTMLElement>("[data-stem-player-status]");
  const canvas = root.querySelector<HTMLCanvasElement>("[data-stem-wave]");
  const waveWrap = root.querySelector<HTMLElement>("[data-stem-wave-wrap]");

  const noop: StemDemoPlayerHandle = { setAudible() {}, dispose() {} };
  if (!playBtn || !canvas || !waveWrap) return noop;

  const peaksLayer = document.createElement("canvas");
  let colors = readPalette(root);
  let layerKey = "";
  let wrapW = waveWrap.clientWidth;
  let wrapH = waveWrap.clientHeight;
  let lastPeaks: Float32Array | null = null;
  let lastStatus: StemDemoSnapshot["status"] | "" = "";
  let lastTitle = "";
  let lastRemaining = "";
  let lastDuration = "";
  let lastPlayingLabel = "";
  let lastAriaKey = "";
  let lastPlaying = false;
  let resumeAfterScrub = false;
  let disposed = false;

  const refreshColors = () => {
    colors = readPalette(root);
    layerKey = "";
  };

  const ensurePeaksLayer = (peaks: Float32Array | null) => {
    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    const width = Math.max(1, Math.round(wrapW * dpr));
    const height = Math.max(1, Math.round(wrapH * dpr));
    const key = `${width}x${height}|${colors.wave}|${colors.bg}`;
    if (
      layerKey === key &&
      peaks === lastPeaks &&
      peaksLayer.width === width &&
      peaksLayer.height === height
    ) {
      return;
    }
    peaksLayer.width = width;
    peaksLayer.height = height;
    paintPeaksLayer(peaksLayer, peaks, colors);
    lastPeaks = peaks;
    layerKey = key;
  };

  const updateSeekAria = (snap: StemDemoSnapshot) => {
    const ready = snap.duration > 0;
    const max = ready ? Math.max(1, Math.round(snap.duration)) : 1;
    const now = ready ? Math.max(0, Math.min(max, Math.round(snap.position))) : 0;
    const key = `${ready}|${now}|${max}`;
    if (key === lastAriaKey) return;
    lastAriaKey = key;
    if (ready) {
      waveWrap.removeAttribute("aria-disabled");
    } else {
      waveWrap.setAttribute("aria-disabled", "true");
    }
    waveWrap.setAttribute("aria-valuemin", "0");
    waveWrap.setAttribute("aria-valuemax", String(max));
    waveWrap.setAttribute("aria-valuenow", String(now));
    waveWrap.setAttribute(
      "aria-valuetext",
      `${formatDeckTime(snap.position)} of ${formatDeckTime(snap.duration)}`,
    );
  };

  let player: StemDemoPlayer;
  const paint = (snap: StemDemoSnapshot) => {
    if (disposed) return;
    if (titleEl && snap.title !== lastTitle) {
      lastTitle = snap.title;
      titleEl.textContent = snap.title;
    }

    const remaining = Math.max(0, snap.duration - snap.position);
    const remainingText = formatDeckTime(remaining, true);
    const durationText = formatDeckTime(snap.duration);
    if (remainingEl && remainingText !== lastRemaining) {
      lastRemaining = remainingText;
      remainingEl.textContent = remainingText;
    }
    if (durationEl && durationText !== lastDuration) {
      lastDuration = durationText;
      durationEl.textContent = durationText;
    }

    const playing = snap.status === "playing";
    const playingLabel = playing ? "Pause" : "Play";
    if (playingLabel !== lastPlayingLabel) {
      lastPlayingLabel = playingLabel;
      playBtn.dataset.playing = String(playing);
      playBtn.setAttribute("aria-label", playingLabel);
    }
    if (playing !== lastPlaying) {
      lastPlaying = playing;
      opts.onPlayingChange?.(playing);
    }
    playBtn.disabled = snap.status === "loading";

    if (statusEl && snap.status !== lastStatus) {
      lastStatus = snap.status;
      if (snap.status === "loading") {
        statusEl.hidden = false;
        statusEl.textContent = "Loading stem…";
      } else if (snap.status === "missing" || snap.status === "error") {
        statusEl.hidden = false;
        statusEl.textContent = snap.message ?? STEM_UNAVAILABLE_MESSAGE;
      } else {
        statusEl.hidden = true;
      }
    }

    ensurePeaksLayer(player.getPeaks());
    drawStemWaveform(canvas, peaksLayer, snap.position, snap.duration, colors.playhead);
    updateSeekAria(snap);
  };

  player = new StemDemoPlayer({ onChange: paint });
  paint(player.snapshot());

  const ac = new AbortController();
  const { signal } = ac;

  playBtn.addEventListener(
    "click",
    () => {
      player.primeAudio();
      void player.toggle();
    },
    { signal },
  );

  const seekFromEvent = (clientX: number) => {
    if (wrapW <= 0) return;
    const rect = waveWrap.getBoundingClientRect();
    const ratio = Math.max(0, Math.min(1, (clientX - rect.left) / rect.width));
    player.seekRatio(ratio);
  };

  const endScrub = (event: PointerEvent, completed: boolean) => {
    if (waveWrap.hasPointerCapture(event.pointerId)) {
      waveWrap.releasePointerCapture(event.pointerId);
    }
    const shouldResume = resumeAfterScrub;
    resumeAfterScrub = false;
    if (!completed || !shouldResume || disposed) return;
    player.primeAudio();
    void player.play();
  };

  waveWrap.addEventListener(
    "pointerdown",
    (event) => {
      if (event.button !== 0) return;
      if (player.snapshot().duration <= 0) return;
      // Pause for the drag so pointermove seeks don't rebuild sources every event.
      resumeAfterScrub = player.snapshot().status === "playing";
      if (resumeAfterScrub) player.pause();
      waveWrap.setPointerCapture(event.pointerId);
      seekFromEvent(event.clientX);
    },
    { signal },
  );
  waveWrap.addEventListener(
    "pointermove",
    (event) => {
      if (!waveWrap.hasPointerCapture(event.pointerId)) return;
      seekFromEvent(event.clientX);
    },
    { signal },
  );
  waveWrap.addEventListener("pointerup", (event) => endScrub(event, true), { signal });
  waveWrap.addEventListener("pointercancel", (event) => endScrub(event, false), { signal });
  waveWrap.addEventListener(
    "keydown",
    (event) => {
      const snap = player.snapshot();
      if (snap.duration <= 0) return;
      const step = Math.max(1, snap.duration * 0.05);
      if (event.key === "ArrowLeft" || event.key === "ArrowDown") {
        event.preventDefault();
        player.seek(snap.position - step);
      } else if (event.key === "ArrowRight" || event.key === "ArrowUp") {
        event.preventDefault();
        player.seek(snap.position + step);
      } else if (event.key === "Home") {
        event.preventDefault();
        player.seek(0);
      } else if (event.key === "End") {
        event.preventDefault();
        player.seek(snap.duration);
      }
    },
    { signal },
  );

  const ro = new ResizeObserver((entries) => {
    const box = entries[0]?.contentRect;
    wrapW = box?.width ?? waveWrap.clientWidth;
    wrapH = box?.height ?? waveWrap.clientHeight;
    layerKey = "";
    paint(player.snapshot());
  });
  ro.observe(waveWrap);

  const themeObs = new MutationObserver(() => {
    refreshColors();
    paint(player.snapshot());
  });
  themeObs.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ["data-theme"],
  });

  const dispose = () => {
    if (disposed) return;
    disposed = true;
    resumeAfterScrub = false;
    if (lastPlaying) {
      lastPlaying = false;
      opts.onPlayingChange?.(false);
    }
    ac.abort();
    ro.disconnect();
    themeObs.disconnect();
    player.dispose();
    liveHandles.delete(handle);
    if (handlesByRoot.get(root) === handle) handlesByRoot.delete(root);
  };

  const handle: StemDemoPlayerHandle = {
    setAudible(audible) {
      player.setAudible(audible);
    },
    dispose,
  };
  handlesByRoot.set(root, handle);
  liveHandles.add(handle);
  return handle;
}
