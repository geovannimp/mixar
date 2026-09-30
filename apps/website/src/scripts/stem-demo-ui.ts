import {
  formatDeckTime,
  StemDemoPlayer,
  type StemDemoSnapshot,
} from "./stem-demo-player";

/** Paint a mirrored overview waveform + playhead onto the demo canvas. */
export function drawStemWaveform(
  canvas: HTMLCanvasElement,
  peaks: Float32Array | null,
  position: number,
  duration: number,
  colors: { wave: string; playhead: string; bg: string },
): void {
  const rect = canvas.getBoundingClientRect();
  const dpr = Math.min(window.devicePixelRatio || 1, 2);
  const width = Math.max(1, Math.round(rect.width * dpr));
  const height = Math.max(1, Math.round(rect.height * dpr));
  if (canvas.width !== width || canvas.height !== height) {
    canvas.width = width;
    canvas.height = height;
  }

  const ctx = canvas.getContext("2d");
  if (!ctx) return;

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

  if (duration > 0) {
    const x = (position / duration) * width;
    ctx.strokeStyle = colors.playhead;
    ctx.lineWidth = Math.max(1, dpr);
    ctx.beginPath();
    ctx.moveTo(x, 0);
    ctx.lineTo(x, height);
    ctx.stroke();
  }
}

function readThemeColor(el: Element, name: string, fallback: string): string {
  const value = getComputedStyle(el).getPropertyValue(name).trim();
  return value || fallback;
}

/**
 * Bind the deck-style transport panel inside a stems demo root.
 * Returns `setAudible` so mute/solo can drive stem gains.
 */
export function initStemDemoPlayer(root: HTMLElement): {
  setAudible: (audible: boolean[]) => void;
} {
  const playBtn = root.querySelector<HTMLButtonElement>("[data-stem-play]");
  const titleEl = root.querySelector<HTMLElement>("[data-stem-title]");
  const keyEl = root.querySelector<HTMLElement>("[data-stem-key]");
  const elapsedEl = root.querySelector<HTMLElement>("[data-stem-elapsed]");
  const durationEl = root.querySelector<HTMLElement>("[data-stem-duration]");
  const statusEl = root.querySelector<HTMLElement>("[data-stem-player-status]");
  const canvas = root.querySelector<HTMLCanvasElement>("[data-stem-wave]");
  const waveWrap = root.querySelector<HTMLElement>("[data-stem-wave-wrap]");

  if (!playBtn || !canvas || !waveWrap) {
    return { setAudible() {} };
  }

  const paint = (snap: StemDemoSnapshot) => {
    if (titleEl) titleEl.textContent = snap.title;
    if (keyEl) keyEl.textContent = snap.key;
    const remaining = Math.max(0, snap.duration - snap.position);
    if (elapsedEl) elapsedEl.textContent = formatDeckTime(remaining, true);
    if (durationEl) durationEl.textContent = formatDeckTime(snap.duration);

    const playing = snap.status === "playing";
    playBtn.dataset.playing = String(playing);
    playBtn.setAttribute("aria-label", playing ? "Pause" : "Play");
    playBtn.disabled = snap.status === "loading";

    if (statusEl) {
      if (snap.status === "loading") {
        statusEl.hidden = false;
        statusEl.textContent = "Loading stem…";
      } else if (snap.status === "missing" || snap.status === "error") {
        statusEl.hidden = false;
        statusEl.textContent = snap.message ?? "Stem unavailable";
      } else {
        statusEl.hidden = true;
      }
    }

    const wave = readThemeColor(root, "--accent", "#2dd4bf");
    drawStemWaveform(canvas, player.getPeaks(), snap.position, snap.duration, {
      wave,
      playhead: "#f4f4f5",
      bg: "#000000",
    });
  };

  const player = new StemDemoPlayer({ onChange: paint });
  paint(player.snapshot());

  // Warm the fetch so Play is snappy, but don't block first paint on decode.
  void player.ensureLoaded();

  playBtn.addEventListener("click", () => {
    void player.toggle();
  });

  const seekFromEvent = (clientX: number) => {
    const rect = waveWrap.getBoundingClientRect();
    if (rect.width <= 0) return;
    const ratio = Math.max(0, Math.min(1, (clientX - rect.left) / rect.width));
    player.seekRatio(ratio);
  };

  waveWrap.addEventListener("pointerdown", (event) => {
    if (player.snapshot().duration <= 0) return;
    waveWrap.setPointerCapture(event.pointerId);
    seekFromEvent(event.clientX);
  });
  waveWrap.addEventListener("pointermove", (event) => {
    if (!waveWrap.hasPointerCapture(event.pointerId)) return;
    seekFromEvent(event.clientX);
  });

  const ro = new ResizeObserver(() => paint(player.snapshot()));
  ro.observe(waveWrap);

  return {
    setAudible(audible) {
      player.setAudible(audible);
    },
  };
}
