/**
 * In-browser NI Stem demo player: extract with stem-mp4, mix four stems via Web Audio.
 * Asset path is fixed so the marketing page can ship before the demo file exists.
 */

/** Public URL for the demo `.stem.mp4` (drop the file under `public/demo/`). */
export const STEM_DEMO_URL = "/demo/track.stem.mp4";

/** stem-mp4 track indices: 0 master, 1–4 drums/bass/other/vocals. */
const STEM_TRACK_INDICES = [1, 2, 3, 4] as const;

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
  key: string;
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
    const end = Math.min(channel.length, Math.floor((i + 1) * bucket));
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
  return track.buffer.slice(track.byteOffset, track.byteOffset + track.byteLength);
}

export class StemDemoPlayer {
  readonly url: string;
  readonly peaksSampleCount = 600;

  private readonly onChange?: (snap: StemDemoSnapshot) => void;
  private title: string;
  private key = "—";
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
  private raf = 0;
  private audible = [true, true, true, true];

  constructor(opts: StemDemoPlayerOptions = {}) {
    this.url = opts.url ?? STEM_DEMO_URL;
    this.title = opts.title ?? "track.stem.mp4";
    this.onChange = opts.onChange;
  }

  getPeaks(): Float32Array | null {
    return this.peaks;
  }

  snapshot(): StemDemoSnapshot {
    return {
      status: this.playing ? "playing" : this.status,
      title: this.title,
      key: this.key,
      position: this.position(),
      duration: this.duration,
      message: this.message,
    };
  }

  position(): number {
    if (this.playing && this.ctx) {
      return Math.min(this.pauseAt + (this.ctx.currentTime - this.startedAt), this.duration);
    }
    return this.pauseAt;
  }

  /** Wire mute/solo: `true` = audible. */
  setAudible(audible: boolean[]): void {
    for (let i = 0; i < 4; i++) {
      this.audible[i] = audible[i] ?? true;
      const gain = this.gains[i];
      if (gain && this.ctx) {
        gain.gain.setValueAtTime(this.audible[i] ? 1 : 0, this.ctx.currentTime);
      }
    }
  }

  async ensureLoaded(): Promise<void> {
    if (this.status === "ready" || this.status === "playing" || this.status === "paused") {
      return;
    }
    if (this.loadPromise) return this.loadPromise;
    this.loadPromise = this.load();
    try {
      await this.loadPromise;
    } finally {
      this.loadPromise = null;
    }
  }

  async toggle(): Promise<void> {
    if (this.playing) {
      this.pause();
      return;
    }
    await this.play();
  }

  async play(): Promise<void> {
    await this.ensureLoaded();
    if (this.status === "missing" || this.status === "error") return;
    if (!this.ctx || this.buffers.length === 0) return;

    if (this.ctx.state === "suspended") await this.ctx.resume();

    if (this.pauseAt >= this.duration && this.duration > 0) {
      this.pauseAt = 0;
    }

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
    if (this.duration <= 0) return;
    const next = Math.max(0, Math.min(seconds, this.duration));
    const wasPlaying = this.playing;
    this.stopSources();
    this.playing = false;
    this.pauseAt = next;
    this.status = wasPlaying ? "playing" : this.status === "ready" ? "paused" : this.status;
    this.emit();
    if (wasPlaying) {
      void this.play();
    }
  }

  seekRatio(ratio: number): void {
    this.seek(ratio * this.duration);
  }

  dispose(): void {
    this.pause();
    this.stopTick();
    void this.ctx?.close();
    this.ctx = null;
  }

  private async load(): Promise<void> {
    this.status = "loading";
    this.message = undefined;
    this.emit();

    let response: Response;
    try {
      response = await fetch(this.url);
    } catch {
      this.status = "error";
      this.message = "Could not fetch demo stem";
      this.emit();
      return;
    }

    if (response.status === 404) {
      this.status = "missing";
      this.message = "Drop track.stem.mp4 into public/demo/";
      this.emit();
      return;
    }
    if (!response.ok) {
      this.status = "error";
      this.message = `Stem fetch failed (${response.status})`;
      this.emit();
      return;
    }

    try {
      const data = new Uint8Array(await response.arrayBuffer());
      const { extractAllTracks } = await import("stem-mp4/extractor");
      const tracks = extractAllTracks(data);
      if (tracks.length < 5) {
        this.status = "error";
        this.message = "File is not a 5-track Stem";
        this.emit();
        return;
      }

      this.ctx ??= new AudioContext();
      const stemBuffers: AudioBuffer[] = [];
      for (const index of STEM_TRACK_INDICES) {
        const track = tracks[index];
        if (!track) throw new Error(`missing stem track ${index}`);
        stemBuffers.push(await this.ctx.decodeAudioData(copyTrackBytes(track)));
      }

      const master = tracks[0];
      const overview = master
        ? await this.ctx.decodeAudioData(copyTrackBytes(master))
        : stemBuffers[0]!;

      this.buffers = stemBuffers;
      this.duration = Math.max(...stemBuffers.map((b) => b.duration));
      this.peaks = peaksFromBuffer(overview, this.peaksSampleCount);
      this.gains = stemBuffers.map((_, i) => {
        const gain = this.ctx!.createGain();
        gain.gain.value = this.audible[i] ? 1 : 0;
        gain.connect(this.ctx!.destination);
        return gain;
      });
      this.pauseAt = 0;
      this.status = "ready";
      this.message = undefined;
      this.emit();
    } catch (err) {
      this.status = "error";
      this.message = err instanceof Error ? err.message : "Failed to decode stem";
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
