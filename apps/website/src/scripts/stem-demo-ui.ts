import {
  formatDeckTime,
  StemDemoPlayer,
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
export function drawStemWaveform(
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

export interface StemDemoPlayerHandle {
  setAudible: (audible: boolean[]) => void;
  dispose: () => void;
}

/**
 * Bind the deck-style transport panel inside a stems demo root.
 * Load is deferred until the first Play so the marketing page stays light.
 */
export function initStemDemoPlayer(root: HTMLElement): StemDemoPlayerHandle {
  const playBtn = root.querySelector<HTMLButtonElement>("[data-stem-play]");
  const titleEl = root.querySelector<HTMLElement>("[data-stem-title]");
  const elapsedEl = root.querySelector<HTMLElement>("[data-stem-elapsed]");
  const durationEl = root.querySelector<HTMLElement>("[data-stem-duration]");
  const statusEl = root.querySelector<HTMLElement>("[data-stem-player-status]");
  const canvas = root.querySelector<HTMLCanvasElement>("[data-stem-wave]");
  const waveWrap = root.querySelector<HTMLElement>("[data-stem-wave-wrap]");

  const noop: StemDemoPlayerHandle = { setAudible() {}, dispose() {} };
  if (!playBtn || !canvas || !waveWrap) return noop;

  const peaksLayer = document.createElement("canvas");
  let colors: WaveColors = {
    wave: readThemeColor(root, "--accent", "#2dd4bf"),
    playhead: "#f4f4f5",
    bg: "#000000",
  };
  let layerKey = "";
  let lastCssW = 0;
  let lastCssH = 0;
  let lastPeaks: Float32Array | null = null;
  let lastStatus: StemDemoSnapshot["status"] | "" = "";
  let disposed = false;

  const refreshColors = () => {
    colors = {
      wave: readThemeColor(root, "--accent", "#2dd4bf"),
      playhead: "#f4f4f5",
      bg: "#000000",
    };
    layerKey = "";
  };

  const ensurePeaksLayer = (peaks: Float32Array | null, cssW: number, cssH: number) => {
    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    const width = Math.max(1, Math.round(cssW * dpr));
    const height = Math.max(1, Math.round(cssH * dpr));
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
    const max = Math.max(0, Math.round(snap.duration));
    const now = Math.max(0, Math.min(max, Math.round(snap.position)));
    waveWrap.setAttribute("aria-valuemin", "0");
    waveWrap.setAttribute("aria-valuemax", String(max));
    waveWrap.setAttribute("aria-valuenow", String(now));
    waveWrap.setAttribute(
      "aria-valuetext",
      `${formatDeckTime(snap.position)} of ${formatDeckTime(snap.duration)}`,
    );
  };

  const paint = (snap: StemDemoSnapshot) => {
    if (titleEl) titleEl.textContent = snap.title;
    const remaining = Math.max(0, snap.duration - snap.position);
    if (elapsedEl) elapsedEl.textContent = formatDeckTime(remaining, true);
    if (durationEl) durationEl.textContent = formatDeckTime(snap.duration);

    const playing = snap.status === "playing";
    playBtn.dataset.playing = String(playing);
    playBtn.setAttribute("aria-label", playing ? "Pause" : "Play");
    playBtn.disabled = snap.status === "loading";

    if (statusEl && snap.status !== lastStatus) {
      lastStatus = snap.status;
      if (snap.status === "loading") {
        statusEl.hidden = false;
        statusEl.textContent = "Loading stem…";
      } else if (snap.status === "missing" || snap.status === "error") {
        statusEl.hidden = false;
        statusEl.textContent = snap.message ?? "Demo stem unavailable";
      } else {
        statusEl.hidden = true;
      }
    }

    const cssW = waveWrap.clientWidth;
    const cssH = waveWrap.clientHeight;
    if (cssW !== lastCssW || cssH !== lastCssH) {
      lastCssW = cssW;
      lastCssH = cssH;
      layerKey = "";
    }

    const peaks = player.getPeaks();
    ensurePeaksLayer(peaks, cssW, cssH);
    drawStemWaveform(canvas, peaksLayer, snap.position, snap.duration, colors.playhead);
    updateSeekAria(snap);
  };

  const player = new StemDemoPlayer({ onChange: paint });
  paint(player.snapshot());

  const ac = new AbortController();
  const { signal } = ac;

  playBtn.addEventListener("click", () => void player.toggle(), { signal });

  const seekFromEvent = (clientX: number) => {
    const rect = waveWrap.getBoundingClientRect();
    if (rect.width <= 0) return;
    const ratio = Math.max(0, Math.min(1, (clientX - rect.left) / rect.width));
    player.seekRatio(ratio);
  };

  waveWrap.addEventListener(
    "pointerdown",
    (event) => {
      if (player.snapshot().duration <= 0) return;
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

  const ro = new ResizeObserver(() => paint(player.snapshot()));
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
    ac.abort();
    ro.disconnect();
    themeObs.disconnect();
    player.dispose();
  };
  window.addEventListener("pagehide", dispose, { once: true, signal });

  return {
    setAudible(audible) {
      player.setAudible(audible);
    },
    dispose,
  };
}
