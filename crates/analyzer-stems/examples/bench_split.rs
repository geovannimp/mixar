//! Offline stem-split benchmark: real WAV in, x-realtime and output hashes out.
//!
//! ```text
//! cargo run --release -p analyzer-stems --manifest-path crates/Cargo.toml \
//!   --example bench_split -- \
//!   --audio samples/Z8phyR*.wav --seconds 60 --dump /tmp/stems-baseline
//! ```
//!
//! `--dump` writes the four interleaved stereo stems as raw `f32` little-endian
//! plus a `sha256` file, so two builds can be compared for byte-identical output.
//! `--live [--preload]` measures the live producer (time-to-first-window and
//! sustained throughput) instead of the offline split.
//! Force an execution provider with `MIXAR_STEMS_ORT_EP=cpu|webgpu`.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use analyzer_stems::{
    resolve_model, separate_interleaved, split_interleaved_stereo, start_live_separator,
    StemSplitRequest, StemSplitResult, DEFAULT_MODEL,
};
use anyhow::{anyhow, bail, Context, Result};
use audio_core::{LiveStemRing, LoadedAudio, DEFAULT_LIVE_STEM_CAPACITY};
use sha2::{Digest, Sha256};

struct Args {
    audio: PathBuf,
    seconds: f64,
    models_root: PathBuf,
    model: String,
    dump: Option<PathBuf>,
    weights: Option<PathBuf>,
    repeat: usize,
    live: bool,
    preload: bool,
}

fn parse_args() -> Result<Args> {
    let mut audio = None;
    let mut seconds = 60.0;
    let mut models_root = std::env::var_os("HOME")
        .map(|home| Path::new(&home).join(".local/share/top.mixar.app/models"))
        .unwrap_or_else(|| PathBuf::from("models"));
    let mut model = DEFAULT_MODEL.to_string();
    let mut dump = None;
    let mut weights = None;
    let mut repeat = 1usize;
    let mut live = false;
    let mut preload = false;

    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        let mut value = || it.next().ok_or_else(|| anyhow!("{arg} needs a value"));
        match arg.as_str() {
            "--audio" => audio = Some(PathBuf::from(value()?)),
            "--seconds" => seconds = value()?.parse().context("--seconds")?,
            "--models-root" => models_root = PathBuf::from(value()?),
            "--model" => model = value()?,
            "--dump" => dump = Some(PathBuf::from(value()?)),
            "--weights" => weights = Some(PathBuf::from(value()?)),
            "--repeat" => repeat = value()?.parse().context("--repeat")?,
            "--live" => live = true,
            "--preload" => preload = true,
            other => bail!("unknown argument '{other}'"),
        }
    }

    Ok(Args {
        audio: audio.ok_or_else(|| anyhow!("--audio <path.wav> is required"))?,
        seconds,
        models_root,
        model,
        dump,
        weights,
        repeat,
        live,
        preload,
    })
}

fn main() -> Result<()> {
    let args = parse_args()?;
    let (pcm, sample_rate) = read_wav(&args.audio)?;
    let frames = ((args.seconds * f64::from(sample_rate)) as usize).min(pcm.len() / 2);
    let pcm = &pcm[..frames * 2];

    println!(
        "audio={} duration_s={:.2} rate={} frames={} ep={:?}",
        args.audio.display(),
        frames as f64 / f64::from(sample_rate),
        sample_rate,
        frames,
        std::env::var("MIXAR_STEMS_ORT_EP").ok()
    );

    if args.live {
        return run_live(&args, pcm, sample_rate, frames);
    }

    let mut result = None;
    let mut chunk_starts: Vec<Instant> = Vec::new();
    for run in 0..args.repeat.max(1) {
        let starts = std::sync::Mutex::new(Vec::new());
        let started = Instant::now();
        let (stems, backend) = match &args.weights {
            Some(weights) => {
                let (stems, ep) = separate_interleaved(
                    weights,
                    pcm,
                    sample_rate,
                    Some(&|done, _total| {
                        if done > 0 {
                            starts.lock().expect("chunk times").push(Instant::now());
                        }
                    }),
                )?;
                (stems, format!("bench/{ep}"))
            }
            None => {
                let split = split_interleaved_stereo(StemSplitRequest {
                    interleaved_stereo: pcm,
                    sample_rate,
                    models_root: &args.models_root,
                    model_name: &args.model,
                    on_window_progress: Some(Box::new(|done, _total| {
                        if done > 0 {
                            starts.lock().expect("chunk times").push(Instant::now());
                        }
                    })),
                })?;
                (split.stems, split.backend)
            }
        };
        let elapsed = started.elapsed();
        let starts = starts.into_inner().expect("chunk times");
        let mut last_chunk_starts = starts.clone();
        last_chunk_starts.sort_unstable();
        let audio_time = Duration::from_secs_f64(frames as f64 / f64::from(sample_rate));
        let realtime = audio_time.as_secs_f64() / elapsed.as_secs_f64();
        println!(
            "run={run} backend={backend} chunks={} total_ms={} audio_s={:.2} realtime_x={:.3}",
            starts.len(),
            elapsed.as_millis(),
            audio_time.as_secs_f64(),
            realtime
        );
        result = Some(stems);
        chunk_starts = last_chunk_starts;
    }
    let result = result.expect("at least one run");
    let result = StemSplitResult {
        stems: result,
        sample_rate: 44_100,
        backend: String::new(),
    };
    if chunk_starts.len() > 1 {
        let mut gaps: Vec<u128> = chunk_starts
            .windows(2)
            .map(|w| w[1].duration_since(w[0]).as_millis())
            .collect();
        gaps.sort_unstable();
        println!(
            "chunk_ms min={} p50={} max={}",
            gaps[0],
            gaps[gaps.len() / 2],
            gaps[gaps.len() - 1]
        );
    }

    for (i, stem) in result.stems.iter().enumerate() {
        let mut hasher = Sha256::new();
        hasher.update(bytemuck_f32(stem));
        println!(
            "stem[{}] samples={} peak={:.4} sha256={:x}",
            i,
            stem.len(),
            stem.iter().fold(0.0f32, |m, s| m.max(s.abs())),
            hasher.finalize()
        );
    }

    if let Some(dir) = &args.dump {
        std::fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
        let mut manifest = String::new();
        for (i, stem) in result.stems.iter().enumerate() {
            let path = dir.join(format!("stem{i}.f32"));
            std::fs::write(&path, bytemuck_f32(stem))
                .with_context(|| format!("write {}", path.display()))?;
            let mut hasher = Sha256::new();
            hasher.update(bytemuck_f32(stem));
            manifest.push_str(&format!("stem{i} {:x}\n", hasher.finalize()));
            println!("dump={}", path.display());
        }
        std::fs::write(dir.join("sha256"), manifest)?;
    }

    Ok(())
}

/// Measure the live producer: time-to-first-window and sustained throughput.
fn run_live(args: &Args, pcm: &[f32], sample_rate: u32, frames: usize) -> Result<()> {
    let handle = resolve_model(&args.models_root, &args.model, None)?;
    let ring = std::sync::Arc::new(LiveStemRing::new(
        analyzer_stems::STEM_SAMPLE_RATE,
        DEFAULT_LIVE_STEM_CAPACITY,
    ));
    let audio = std::sync::Arc::new(LoadedAudio {
        samples: pcm.to_vec(),
        sample_rate,
        channels: 2,
        source_id: "bench".into(),
    });
    if args.preload {
        let preload_started = Instant::now();
        let ep = analyzer_stems::preload_session(&handle.local_path)?;
        println!(
            "preload ep={ep} ms={}",
            preload_started.elapsed().as_millis()
        );
    }
    let started = Instant::now();
    let separator = start_live_separator(handle.local_path, audio, std::sync::Arc::clone(&ring))?;
    ring.set_playhead(0);

    let mut first_window = None;
    let mut last_end = 0u64;
    let mut window_times = Vec::new();
    loop {
        if let Some((start, end)) = ring.coverage() {
            if first_window.is_none() && end > start {
                first_window = Some(started.elapsed());
            }
            if end as usize >= frames {
                break;
            }
            if end > last_end {
                last_end = end;
                window_times.push(started.elapsed());
                // Let the producer keep going at full speed.
                ring.set_playhead(end.saturating_sub(1));
            }
        }
        if started.elapsed() > Duration::from_secs(1800) {
            anyhow::bail!("live producer timed out");
        }
        if ring.is_closed() {
            anyhow::bail!("live producer closed early");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let total = started.elapsed();
    separator.join();

    let audio_time =
        Duration::from_secs_f64(frames as f64 / f64::from(analyzer_stems::STEM_SAMPLE_RATE));
    if window_times.len() > 2 {
        let mut gaps: Vec<u128> = window_times
            .windows(2)
            .map(|pair| pair[1].saturating_sub(pair[0]).as_millis())
            .collect();
        gaps.sort_unstable();
        println!(
            "live window_ms min={} p50={} max={}",
            gaps[0],
            gaps[gaps.len() / 2],
            gaps[gaps.len() - 1]
        );
    }
    println!(
        "live first_window_ms={} total_ms={} audio_s={:.2} realtime_x={:.3}",
        first_window.unwrap_or_default().as_millis(),
        total.as_millis(),
        audio_time.as_secs_f64(),
        audio_time.as_secs_f64() / total.as_secs_f64()
    );
    Ok(())
}

fn bytemuck_f32(samples: &[f32]) -> Vec<u8> {
    samples.iter().flat_map(|s| s.to_le_bytes()).collect()
}

/// Minimal RIFF/WAVE reader: PCM 16/24/32-bit and IEEE float32, any channel count.
fn read_wav(path: &Path) -> Result<(Vec<f32>, u32)> {
    let bytes = std::fs::read(path).with_context(|| format!("read {}", path.display()))?;
    if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        bail!("{} is not a RIFF/WAVE file", path.display());
    }

    let mut pos = 12;
    let mut format = None;
    let mut channels = 0usize;
    let mut sample_rate = 0u32;
    let mut bits = 0u16;
    let mut data = None;

    while pos + 8 <= bytes.len() {
        let id = &bytes[pos..pos + 4];
        let len = u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into()?) as usize;
        let body = pos + 8;
        let end = (body + len).min(bytes.len());
        match id {
            b"fmt " => {
                if end - body < 16 {
                    bail!("short fmt chunk");
                }
                format = Some(u16::from_le_bytes(bytes[body..body + 2].try_into()?));
                channels = u16::from_le_bytes(bytes[body + 2..body + 4].try_into()?) as usize;
                sample_rate = u32::from_le_bytes(bytes[body + 4..body + 8].try_into()?);
                bits = u16::from_le_bytes(bytes[body + 14..body + 16].try_into()?);
            }
            b"data" => data = Some(&bytes[body..end]),
            _ => {}
        }
        pos = body + len + (len & 1);
    }

    let data = data.ok_or_else(|| anyhow!("no data chunk"))?;
    let channels = channels.max(1);
    let width = usize::from(bits / 8);
    if width == 0 || data.len() < width {
        bail!("unsupported WAV bit depth {bits}");
    }

    let frames = data.len() / (width * channels);
    let mut pcm = Vec::with_capacity(frames * 2);
    let mut frame_buf = vec![0.0f32; channels];
    for frame in 0..frames {
        for (ch, slot) in frame_buf.iter_mut().enumerate() {
            let offset = (frame * channels + ch) * width;
            *slot = match (format, width) {
                (Some(3), 4) => f32::from_le_bytes(data[offset..offset + 4].try_into()?),
                (Some(3), 8) => f64::from_le_bytes(data[offset..offset + 8].try_into()?) as f32,
                (_, 2) => {
                    f32::from(i16::from_le_bytes(data[offset..offset + 2].try_into()?)) / 32768.0
                }
                (_, 3) => {
                    let b = &data[offset..offset + 3];
                    let v = i32::from_le_bytes([
                        b[0],
                        b[1],
                        b[2],
                        if b[2] & 0x80 != 0 { 0xff } else { 0 },
                    ]);
                    v as f32 / 8_388_608.0
                }
                (_, 4) => {
                    i32::from_le_bytes(data[offset..offset + 4].try_into()?) as f32
                        / 2_147_483_648.0
                }
                _ => bail!("unsupported WAV sample width {width}"),
            };
        }
        // Interleaved stereo: mono duplicates, >2 channels take the first two.
        pcm.push(frame_buf[0]);
        pcm.push(if channels > 1 {
            frame_buf[1]
        } else {
            frame_buf[0]
        });
    }

    Ok((pcm, sample_rate))
}
