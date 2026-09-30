/**
 * In-browser NI Stem demo player: extract with stem-mp4, mix four stems via Web Audio.
 * Asset path is fixed so the marketing page can ship before the demo file exists.
 */

/** Public URL for the demo `.stem.mp4` (drop the file under `public/demo/`). */
export const STEM_DEMO_URL = "/demo/track.stem.mp4";

/** Title from the NI free Stems pack demo file (`©nam`). */
export const STEM_DEMO_TITLE = "Hittin Hard";

/** Visitor-facing status when the demo asset cannot be loaded. */
export const STEM_UNAVAILABLE_MESSAGE = "Demo stem unavailable";

/** stem-mp4 track indices: 0 master, 1–4 drums/bass/other/vocals. */
const STEM_TRACK_INDICES = [1, 2, 3, 4] as const;
const STEM_COUNT = STEM_TRACK_INDICES.length;
/** Master + four stems. */
const STEM_FILE_TRACK_COUNT = STEM_COUNT + 1;

const GAIN_RAMP_SEC = 0.015;

export type StemDemoStatus =
  | "idle"
  | "loading"
  | "ready"
  | "missing"
  | "error"
  | "playing"
  | "paused";

export interface StemDemoSnapshot {
  status: StemDemoStatus;
  title: string;
  position: number;
  duration: number;
  message?: string;
}

export interface StemDemoPlayerOptions {
  url?: string;
  title?: string;
  onChange?: (snap: StemDemoSnapshot) => void;
}

/** DJ-style time: `m:ss.d`, or remaining as `-m:ss.d`. */
export function formatDeckTime(seconds: number, remaining = false): string {
  if (!Number.isFinite(seconds) || seconds < 0) seconds = 0;
  const sign = remaining ? "-" : "";
  // Round via tenths so 84.8 does not become 24.7999… → `.7`.
  const totalTenths = Math.round(seconds * 10);
  const mins = Math.floor(totalTenths / 600);
  const remTenths = totalTenths - mins * 600;
  const whole = Math.floor(remTenths / 10);
  const tenths = remTenths % 10;
  return `${sign}${mins}:${whole.toString().padStart(2, "0")}.${tenths}`;
}

/** Peak envelope for a mirrored overview waveform. */
export function peaksFromBuffer(buffer: AudioBuffer, samples: number): Float32Array {
  const channel = buffer.getChannelData(0);
  const peaks = new Float32Array(samples);
  if (channel.length === 0 || samples <= 0) return peaks;
  const bucket = channel.length / samples;
  for (let i = 0; i < samples; i++) {
    const start = Math.floor(i * bucket);
    const end = Math.max(start + 1, Math.min(channel.length, Math.floor((i + 1) * bucket)));
    let max = 0;
    for (let j = start; j < end; j++) {
      const v = Math.abs(channel[j] ?? 0);
      if (v > max) max = v;
    }
    peaks[i] = max;
  }
  return peaks;
}

function copyTrackBytes(track: Uint8Array): ArrayBuffer {
// `Uint8Array#buffer` is typed ArrayBufferLike (ArrayBuffer | SharedArrayBuffer)
  // and `slice()` preserves that, so it will not assign to ArrayBuffer on its
  // own. Tracks here come from `new Uint8Array(await response.arrayBuffer())`, so
  // the underlying buffer is always a plain ArrayBuffer.
  return track.buffer.slice(track.byteOffset, track.byteOffset + track.byteLength) as ArrayBuffer;
}

export class StemDemoPlayer {
  readonly url: string;
  readonly peaksSampleCount = 600;

  private readonly onChange?: (snap: StemDemoSnapshot) => void;
  private title: string;
  private status: StemDemoStatus = "idle";
  private message?: string;
  private ctx: AudioContext | null = null;
  private buffers: AudioBuffer[] = [];
  private gains: GainNode[] = [];
  private sources: AudioBufferSourceNode[] = [];
  private peaks: Float32Array | null = null;
  private duration = 0;
  private pauseAt = 0;
  private startedAt = 0;
  private playing = false;
  private loadPromise: Promise<void> | null = null;
  private playPromise: Promise<void> | null = null;
  private fetchAbort: AbortController | null = null;
  private raf = 0;
  private audible = STEM_TRACK_INDICES.map(() => true);
  private disposed = false;

  constructor(opts: StemDemoPlayerOptions = {}) {
    this.url = opts.url ?? STEM_DEMO_URL;
    this.title = opts.title ?? STEM_DEMO_TITLE;
    this.onChange = opts.onChange;
  }

  getPeaks(): Float32Array | null {
    return this.peaks;
  }

  snapshot(): StemDemoSnapshot {
    return {
      status: this.playing ? "playing" : this.status,
      title: this.title,
      position: this.position(),
      duration: this.duration,
      message: this.message,
    };
  }

  position(): number {
    if (this.playing && this.ctx) {
      return Math.max(
        0,
        Math.min(this.pauseAt + (this.ctx.currentTime - this.startedAt), this.duration),
      );
    }
    return this.pauseAt;
  }

  /**
   * Create/resume the AudioContext inside a user gesture so later awaits
   * (fetch/decode) do not leave playback permanently suspended.
   */
  primeAudio(): void {
    this.ctx ??= new AudioContext();
    if (this.ctx.state === "suspended") {
      void this.ctx.resume();
    }
  }

  /** Wire mute/solo: `true` = audible. */
  setAudible(audible: boolean[]): void {
    for (let i = 0; i < STEM_COUNT; i++) {
      const next = audible[i] ?? true;
      if (this.audible[i] === next) continue;
      this.audible[i] = next;
      const gain = this.gains[i];
      if (!gain || !this.ctx) continue;
      const now = this.ctx.currentTime;
      gain.gain.cancelScheduledValues(now);
      gain.gain.setValueAtTime(gain.gain.value, now);
      gain.gain.linearRampToValueAtTime(next ? 1 : 0, now + GAIN_RAMP_SEC);
    }
  }

  async ensureLoaded(): Promise<void> {
    if (this.disposed) return;
    if (this.status === "ready" || this.status === "playing" || this.status === "paused") {
      return;
    }
    // Share one in-flight load across concurrent Play clicks; clear after settle
    // so a later Play can retry a transient missing/error failure.
    this.loadPromise ??= this.load().finally(() => {
      this.loadPromise = null;
    });
    await this.loadPromise;
  }

  async toggle(): Promise<void> {
    if (this.disposed) return;
    if (this.playing) {
      this.pause();
      return;
    }
    await this.play();
  }

  async play(): Promise<void> {
    if (this.disposed) return;
    if (this.playing) return;
    // Collapse rapid double-clicks while the first Play is still awaiting load.
    if (this.playPromise) return this.playPromise;
    this.playPromise = this.startPlayback();
    try {
      await this.playPromise;
    } finally {
      this.playPromise = null;
    }
  }

  private async startPlayback(): Promise<void> {
    await this.ensureLoaded();
    if (this.disposed || this.playing) return;
    if (this.status === "missing" || this.status === "error") return;
    if (!this.ctx || this.buffers.length === 0) return;

    if (this.ctx.state === "suspended") await this.ctx.resume();
    if (this.disposed || this.playing) return;

    if (this.pauseAt >= this.duration && this.duration > 0) {
      this.pauseAt = 0;
    }

    // Cancel any prior RAF chain before starting a new one (seek-while-playing
    // used to orphan frames by overwriting `this.raf`).
    this.stopTick();
    this.stopSources();
    const when = this.ctx.currentTime + 0.03;
    this.startedAt = when;
    this.sources = this.buffers.map((buffer, i) => {
      const source = this.ctx!.createBufferSource();
      source.buffer = buffer;
      source.connect(this.gains[i]!);
      source.start(when, this.pauseAt);
      return source;
    });
    this.playing = true;
    this.status = "playing";
    this.emit();
    this.tick();
  }

  pause(): void {
    if (!this.playing) return;
    this.pauseAt = this.position();
    this.stopSources();
    this.playing = false;
    this.status = "paused";
    this.stopTick();
    this.emit();
  }

  seek(seconds: number): void {
    if (this.disposed || this.duration <= 0) return;
    const next = Math.max(0, Math.min(seconds, this.duration));
    const wasPlaying = this.playing;
    this.stopTick();
    this.stopSources();
    this.playing = false;
    this.pauseAt = next;
    // Seek only works after load (`duration > 0`), so idle never reaches here.
    // Keep paused when scrubbing while stopped; play() restores playing.
    if (!wasPlaying && (this.status === "ready" || this.status === "paused")) {
      this.status = "paused";
    }
    this.emit();
    if (wasPlaying) {
      void this.play();
    }
  }

  seekRatio(ratio: number): void {
    this.seek(ratio * this.duration);
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    this.fetchAbort?.abort();
    this.fetchAbort = null;
    this.pause();
    this.stopTick();
    void this.ctx?.close();
    this.ctx = null;
    this.buffers = [];
    this.gains = [];
    this.sources = [];
    this.peaks = null;
    this.duration = 0;
    this.pauseAt = 0;
    this.loadPromise = null;
    this.playPromise = null;
    this.status = "idle";
    this.message = undefined;
  }

  private async load(): Promise<void> {
    if (this.disposed) return;
    this.status = "loading";
    this.message = undefined;
    this.emit();

    this.fetchAbort?.abort();
    this.fetchAbort = new AbortController();
    const { signal } = this.fetchAbort;

    let response: Response;
    try {
      response = await fetch(this.url, { signal });
    } catch {
      if (this.disposed || signal.aborted) return;
      this.status = "error";
      this.message = STEM_UNAVAILABLE_MESSAGE;
      this.emit();
      return;
    }

    if (this.disposed || signal.aborted) return;

    if (response.status === 404) {
      this.status = "missing";
      this.message = STEM_UNAVAILABLE_MESSAGE;
      console.info("Stem demo: drop track.stem.mp4 into public/demo/");
      this.emit();
      return;
    }
    if (!response.ok) {
      this.status = "error";
      this.message = STEM_UNAVAILABLE_MESSAGE;
      this.emit();
      return;
    }

    try {
      const data = new Uint8Array(await response.arrayBuffer());
      if (this.disposed || signal.aborted) return;
      const { extractAllTracks } = await import("stem-mp4/extractor");
      if (this.disposed || signal.aborted) return;
      const tracks = extractAllTracks(data);
      if (tracks.length < STEM_FILE_TRACK_COUNT) {
        this.status = "error";
        this.message = STEM_UNAVAILABLE_MESSAGE;
        this.emit();
        return;
      }

      this.ctx ??= new AudioContext();
      const ctx = this.ctx;
      const master = tracks[0];
      const [stemBuffers, overview] = await Promise.all([
        Promise.all(
          STEM_TRACK_INDICES.map(async (index) => {
            const track = tracks[index];
            if (!track) throw new Error(`missing stem track ${index}`);
            return ctx.decodeAudioData(copyTrackBytes(track));
          }),
        ),
        master ? ctx.decodeAudioData(copyTrackBytes(master)) : Promise.resolve(null),
      ]);
      if (this.disposed || signal.aborted) return;
      const overviewBuf = overview ?? stemBuffers[0]!;

      this.buffers = stemBuffers;
      this.duration = Math.max(...stemBuffers.map((b) => b.duration));
      this.peaks = peaksFromBuffer(overviewBuf, this.peaksSampleCount);
      this.gains = stemBuffers.map((_, i) => {
        const gain = ctx.createGain();
        gain.gain.value = this.audible[i] ? 1 : 0;
        gain.connect(ctx.destination);
        return gain;
      });
      this.pauseAt = 0;
      this.status = "ready";
      this.message = undefined;
      this.emit();
    } catch {
      if (this.disposed || signal.aborted) return;
      this.status = "error";
      this.message = STEM_UNAVAILABLE_MESSAGE;
      this.emit();
    }
  }

  private stopSources(): void {
    for (const source of this.sources) {
      try {
        source.stop();
        source.disconnect();
      } catch {
        /* already stopped */
      }
    }
    this.sources = [];
  }

  private tick = (): void => {
    if (!this.playing) return;
    const pos = this.position();
    if (pos >= this.duration && this.duration > 0) {
      this.pauseAt = this.duration;
      this.stopSources();
      this.playing = false;
      this.status = "paused";
      this.stopTick();
      this.emit();
      return;
    }
    this.emit();
    this.raf = requestAnimationFrame(this.tick);
  };

  private stopTick(): void {
    if (this.raf) cancelAnimationFrame(this.raf);
    this.raf = 0;
  }

  private emit(): void {
    this.onChange?.(this.snapshot());
  }
}
