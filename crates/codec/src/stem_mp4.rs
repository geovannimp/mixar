use crate::{codec_registry, AudioDecoder, StemAtom};
use anyhow::{anyhow, bail, Context, Result};
use audio_core::Sample;
use flacenc::bitsink::ByteSink;
use flacenc::component::BitRepr;
use flacenc::error::Verify;
use flacenc::source::MemSource;
use mp4io::{Codec, InputSample, TrackParams, Writer, WriterConfig};
use ruopus::{Bandwidth, OpusEncoder};
use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::path::Path;
use symphonia::core::audio::Layout;
use symphonia::core::codecs::{
    CodecParameters, Decoder, DecoderOptions, CODEC_TYPE_NULL, CODEC_TYPE_OPUS,
};
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::{MetadataOptions, MetadataRevision, Value};
use symphonia::core::probe::Hint;

/// Opus is always decoded at 48 kHz, so muxed Opus stems must already be there.
pub const OPUS_SAMPLE_RATE: u32 = 48_000;

/// Target Opus bitrate per stream (matches the old four-file cache).
const OPUS_BITRATE_BPS: u32 = 160_000;

/// 20 ms at 48 kHz — one Opus packet, one MP4 sample.
const OPUS_FRAME: usize = 960;

/// Largest Opus packet ruopus emits for a 20 ms frame.
const OPUS_MAX_PACKET: usize = 1275;

/// Decoder warm-up to discard: the fullband CELT reconstruction delay `ruopus`
/// `encode_auto` incurs above 40 kb/s (it uses 69 for hybrid below that).
const OPUS_PRE_SKIP: u16 = 120;

/// On-disk codec for the five streams of a `.stem.mp4`. No AAC encoder in-tree yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StemMuxFormat {
    Opus,
    Flac,
}

pub struct StemPcmBundle {
    pub sample_rate: u32,
    pub mixdown: Vec<f32>,
    pub stems: [Vec<f32>; 4],
    pub atom: Option<StemAtom>,
}

pub fn is_stem_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.to_ascii_lowercase().ends_with(".stem.mp4"))
}

pub fn decode_stem_file(path: &Path) -> Result<StemPcmBundle> {
    let file = File::open(path)?;
    let source = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(extension) = path.extension().and_then(|extension| extension.to_str()) {
        hint.with_extension(extension);
    }

    let mut format = symphonia::default::get_probe()
        .format(
            &hint,
            source,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )?
        .format;
    let atom = stem_atom_from_udta(path).or_else(|| {
        format
            .metadata()
            .skip_to_latest()
            .and_then(stem_atom_from_metadata)
    });

    let decoder_options = DecoderOptions::default();
    let mut tracks: Vec<(u32, u32, Box<dyn Decoder>)> = format
        .tracks()
        .iter()
        .filter(|track| track.codec_params.codec != CODEC_TYPE_NULL)
        .filter_map(|track| {
            codec_registry()
                .make(&decoder_params(&track.codec_params), &decoder_options)
                .ok()
                .map(|decoder| {
                    (
                        track.id,
                        track.codec_params.sample_rate.unwrap_or(44_100),
                        decoder,
                    )
                })
        })
        .take(5)
        .collect();

    if tracks.len() < 5 {
        bail!(
            "Stem file requires at least 5 decodable audio tracks, found {}",
            tracks.len()
        );
    }

    let sample_rate = tracks[0].1;
    let mut samples: [Vec<Sample>; 5] = std::array::from_fn(|_| Vec::new());
    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(symphonia::core::errors::Error::ResetRequired) => {
                bail!("Stem track list changed while decoding")
            }
            Err(symphonia::core::errors::Error::IoError(_)) => break,
            Err(error) => return Err(error.into()),
        };

        let Some(index) = tracks
            .iter()
            .position(|(track_id, _, _)| *track_id == packet.track_id())
        else {
            continue;
        };
        let decoded = tracks[index].2.decode(&packet)?;
        samples[index].extend(AudioDecoder::audio_buffer_to_samples(&decoded));
    }

    let [mixdown, drums, bass, other, vocals] = samples;
    Ok(StemPcmBundle {
        sample_rate,
        mixdown,
        stems: [drums, bass, other, vocals],
        atom,
    })
}

/// Repair the Opus codec parameters Symphonia's isomp4 reader produces. It hands
/// the `dOps` payload through as extra data with an `OpusHead` magic glued on
/// front, but never reads it, so two fields need fixing for any Opus-in-MP4 file:
/// `channels` is left unset (the decoder refuses to open), and the consumer
/// parses the header as little-endian Ogg `OpusHead` while `dOps` is big-endian
/// ISOBMFF, so pre-skip is read byte-swapped and trims the wrong amount.
fn decoder_params(params: &CodecParameters) -> CodecParameters {
    let mut params = params.clone();
    if params.codec != CODEC_TYPE_OPUS {
        return params;
    }

    // Layout after the 8-byte magic: version, channel count, pre-skip.
    let channels = match params.extra_data.as_mut() {
        Some(extra_data) if extra_data.len() >= 12 => {
            extra_data.swap(10, 11);
            extra_data[9]
        }
        _ => return params,
    };
    if params.channels.is_none() {
        let layout = match channels {
            1 => Layout::Mono,
            2 => Layout::Stereo,
            _ => return params,
        };
        params.with_channels(layout.into_channels());
    }
    params
}

/// Mux five consistent streams (mixdown + NI `drums, bass, other, vocals`) into
/// `path` as an NI `.stem.mp4`, with the STEM JSON manifest in `moov/udta/stem`.
///
/// All five inputs must be interleaved stereo at `sample_rate`; they are
/// truncated to the shortest so every track shares a frame count. Opus requires
/// [`OPUS_SAMPLE_RATE`] — resample before calling. The file is written to a
/// sibling temp file and renamed, so `path` is never left half-written.
///
/// // ponytail: whole file is encoded in RAM before the rename; streaming mux if cache size hurts.
pub fn encode_stem_mp4(
    path: &Path,
    format: StemMuxFormat,
    sample_rate: u32,
    mixdown: &[f32],
    stems: [&[f32]; 4],
    atom: &StemAtom,
) -> Result<()> {
    if sample_rate == 0 {
        bail!("sample_rate must be > 0");
    }
    if format == StemMuxFormat::Opus && sample_rate != OPUS_SAMPLE_RATE {
        bail!("Opus stems must be {OPUS_SAMPLE_RATE} Hz, got {sample_rate}");
    }

    let inputs = [mixdown, stems[0], stems[1], stems[2], stems[3]];
    for pcm in inputs {
        if !pcm.len().is_multiple_of(2) {
            bail!("stem PCM length {} is not interleaved stereo", pcm.len());
        }
    }
    let frames = inputs.iter().map(|pcm| pcm.len() / 2).min().unwrap_or(0);
    if frames == 0 {
        bail!("stem PCM is empty");
    }

    let encoded = inputs
        .iter()
        .map(|pcm| match format {
            StemMuxFormat::Opus => encode_opus(&pcm[..frames * 2]),
            StemMuxFormat::Flac => encode_flac(&pcm[..frames * 2], sample_rate),
        })
        .collect::<Result<Vec<_>>>()?;

    let (codec, track_rate) = match format {
        StemMuxFormat::Opus => (Codec::Opus, OPUS_SAMPLE_RATE),
        StemMuxFormat::Flac => (Codec::Flac, sample_rate),
    };
    let mut writer = Writer::new(WriterConfig::default());
    let tracks: Vec<_> = encoded
        .iter()
        .map(|stream| {
            let params = TrackParams::audio(codec, 2, track_rate).timescale(track_rate);
            writer.add_track(match format {
                StemMuxFormat::Opus => params.opus_config(stream.config.clone()),
                StemMuxFormat::Flac => params.flac_config(stream.config.clone()),
            })
        })
        .collect();

    // mp4io chunks by push order, so interleave roughly a second of each track at
    // a time: contiguous enough to keep `stco`/`stsc` small, interleaved enough
    // that a player does not seek across the whole file per packet.
    let per_chunk = encoded[0].samples.first().map_or(1, |(_, duration)| {
        (track_rate / (*duration).max(1)).max(1) as usize
    });
    let longest = encoded
        .iter()
        .map(|stream| stream.samples.len())
        .max()
        .unwrap_or(0);
    for start in (0..longest).step_by(per_chunk) {
        for (track, stream) in tracks.iter().zip(&encoded) {
            let end = (start + per_chunk).min(stream.samples.len());
            for (data, duration) in &stream.samples[start.min(end)..end] {
                writer.write_sample(
                    *track,
                    InputSample {
                        data,
                        duration: *duration,
                        composition_offset: 0,
                        is_sync: true,
                    },
                )?;
            }
        }
    }

    let bytes = with_stem_udta(writer.finalize()?, &serde_json::to_vec(atom)?)?;
    let temp = path.with_extension("tmp");
    std::fs::write(&temp, &bytes).with_context(|| format!("write {}", temp.display()))?;
    std::fs::rename(&temp, path)
        .inspect_err(|_| {
            let _ = std::fs::remove_file(&temp);
        })
        .with_context(|| format!("rename onto {}", path.display()))
}

/// One muxed track: its codec config box payload plus `(sample, duration)` pairs.
struct EncodedStream {
    config: Vec<u8>,
    samples: Vec<(Vec<u8>, u32)>,
}

fn encode_opus(pcm: &[f32]) -> Result<EncodedStream> {
    let mut encoder = OpusEncoder::new(2);
    encoder.set_bandwidth(Bandwidth::FullBand);
    encoder.set_bitrate(Some(OPUS_BITRATE_BPS));

    let frame_samples = OPUS_FRAME * 2;
    let samples = pcm
        .chunks(frame_samples)
        .map(|chunk| {
            let mut frame = chunk.to_vec();
            frame.resize(frame_samples, 0.0); // zero-pad the final partial frame
            let packet = encoder
                .encode_auto(&frame, OPUS_MAX_PACKET)
                .map_err(|error| anyhow!("opus encode: {error:?}"))?;
            Ok((packet, OPUS_FRAME as u32))
        })
        .collect::<Result<Vec<_>>>()?;

    // `dOps` (Opus in ISOBMFF): version 0, then the OpusHead fields big-endian.
    let mut config = vec![0u8, 2];
    config.extend_from_slice(&OPUS_PRE_SKIP.to_be_bytes());
    config.extend_from_slice(&OPUS_SAMPLE_RATE.to_be_bytes());
    config.extend_from_slice(&0i16.to_be_bytes()); // output gain
    config.push(0); // channel mapping family 0
    Ok(EncodedStream { config, samples })
}

fn encode_flac(pcm: &[f32], sample_rate: u32) -> Result<EncodedStream> {
    let pcm_i32: Vec<i32> = pcm
        .iter()
        .map(|&sample| {
            (sample * f32::from(i16::MAX)).clamp(f32::from(i16::MIN), f32::from(i16::MAX)) as i32
        })
        .collect();
    let config = flacenc::config::Encoder::default()
        .into_verified()
        .map_err(|error| anyhow!("flacenc config: {error:?}"))?;
    let source = MemSource::from_samples(&pcm_i32, 2, 16, sample_rate as usize);
    let stream = flacenc::encode_with_fixed_block_size(&config, source, config.block_size)
        .map_err(|error| anyhow!("flac encode: {error:?}"))?;

    let mut info = ByteSink::new();
    stream
        .stream_info()
        .write(&mut info)
        .map_err(|error| anyhow!("flac stream info: {error:?}"))?;
    let info = info.as_slice();

    // `dfLa`: FullBox(0, 0) then FLAC metadata blocks, STREAMINFO first and last.
    let mut dfla = vec![0u8; 4];
    dfla.push(0x80);
    dfla.extend_from_slice(&u32::try_from(info.len())?.to_be_bytes()[1..]);
    dfla.extend_from_slice(info);

    let samples = (0..stream.frame_count())
        .map(|index| {
            let frame = stream
                .frame(index)
                .ok_or_else(|| anyhow!("flac frame {index} missing"))?;
            let mut sink = ByteSink::new();
            frame
                .write(&mut sink)
                .map_err(|error| anyhow!("flac frame {index}: {error:?}"))?;
            Ok((sink.as_slice().to_vec(), frame.block_size() as u32))
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(EncodedStream {
        config: dfla,
        samples,
    })
}

/// Append `moov/udta/stem` (the NI STEM manifest Traktor and Mixxx read) to a
/// muxed file. `mp4io` writes `moov` last, so growing it moves no sample offset.
fn with_stem_udta(mut mp4: Vec<u8>, json: &[u8]) -> Result<Vec<u8>> {
    let total = mp4.len() as u64;
    let moov = find_atom(&mut Cursor::new(&mp4), 0, total, &[*b"moov"])?
        .context("muxed mp4 has no moov atom")?;
    if moov.start + moov.header_len + moov.payload_len != total {
        bail!("moov is not the last atom; cannot append the STEM atom");
    }
    if moov.header_len != 8 {
        bail!("64-bit moov atom is not supported");
    }

    let udta = atom_box(b"udta", &atom_box(b"stem", json));
    let size = u32::try_from(moov.header_len + moov.payload_len + udta.len() as u64)
        .context("moov atom exceeds 4 GiB")?;
    let start = moov.start as usize;
    mp4[start..start + 4].copy_from_slice(&size.to_be_bytes());
    mp4.extend_from_slice(&udta);
    Ok(mp4)
}

fn atom_box(fourcc: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + payload.len());
    out.extend_from_slice(&(8 + payload.len() as u32).to_be_bytes());
    out.extend_from_slice(fourcc);
    out.extend_from_slice(payload);
    out
}

/// Where an atom sits: box start, header length (8 or 16), payload length.
struct AtomSpan {
    start: u64,
    header_len: u64,
    payload_len: u64,
}

/// Locate a nested atom by fourcc path within `[pos, end)` of a seekable stream.
fn find_atom<R: Read + Seek>(
    reader: &mut R,
    mut pos: u64,
    end: u64,
    path: &[[u8; 4]],
) -> Result<Option<AtomSpan>> {
    let Some((want, rest)) = path.split_first() else {
        return Ok(None);
    };

    while pos + 8 <= end {
        reader.seek(SeekFrom::Start(pos))?;
        let mut header = [0u8; 8];
        reader.read_exact(&mut header)?;
        let fourcc = [header[4], header[5], header[6], header[7]];
        let (size, header_len) =
            match u32::from_be_bytes([header[0], header[1], header[2], header[3]]) {
                0 => (end - pos, 8), // extends to the end of the enclosing box
                1 => {
                    let mut large = [0u8; 8];
                    reader.read_exact(&mut large)?;
                    (u64::from_be_bytes(large), 16)
                }
                size => (u64::from(size), 8),
            };
        if size < header_len || pos + size > end {
            bail!("malformed mp4 atom at offset {pos}");
        }

        if fourcc == *want {
            let span = AtomSpan {
                start: pos,
                header_len,
                payload_len: size - header_len,
            };
            return if rest.is_empty() {
                Ok(Some(span))
            } else {
                let payload = span.start + span.header_len;
                find_atom(reader, payload, payload + span.payload_len, rest)
            };
        }
        pos += size;
    }

    Ok(None)
}

/// Read the NI STEM manifest from `moov/udta/stem` without decoding audio.
fn stem_atom_from_udta(path: &Path) -> Option<StemAtom> {
    /// A manifest is a few hundred bytes; anything larger is not one.
    const MAX_MANIFEST: u64 = 64 * 1024;

    let mut file = File::open(path).ok()?;
    let end = file.metadata().ok()?.len();
    let span = find_atom(&mut file, 0, end, &[*b"moov", *b"udta", *b"stem"]).ok()??;
    if span.payload_len > MAX_MANIFEST {
        return None;
    }

    let mut json = vec![0u8; span.payload_len as usize];
    file.seek(SeekFrom::Start(span.start + span.header_len))
        .ok()?;
    file.read_exact(&mut json).ok()?;
    // Some writers null-pad the manifest, which serde_json will not accept.
    while json.last() == Some(&0) {
        json.pop();
    }
    serde_json::from_slice(&json).ok()
}

fn stem_atom_from_metadata(metadata: &MetadataRevision) -> Option<StemAtom> {
    metadata
        .tags()
        .iter()
        .filter(|tag| tag.key.to_ascii_lowercase().contains("stem"))
        .find_map(|tag| match &tag.value {
            Value::String(json) => serde_json::from_str(json).ok(),
            Value::Binary(json) => serde_json::from_slice(json).ok(),
            _ => None,
        })
        .or_else(|| {
            metadata
                .vendor_data()
                .iter()
                .filter(|data| data.ident.to_ascii_lowercase().contains("stem"))
                .find_map(|data| serde_json::from_slice(&data.data).ok())
        })
}

#[cfg(test)]
mod tests {
    use super::{decode_stem_file, encode_stem_mp4, is_stem_path, StemMuxFormat};
    use crate::StemAtom;
    use std::path::Path;

    #[test]
    fn is_stem_path_detects_extension() {
        assert!(is_stem_path(Path::new("/x/Track.stem.mp4")));
        assert!(is_stem_path(Path::new("/x/Track.STEM.MP4")));
        assert!(!is_stem_path(Path::new("/x/Track.mp4")));
        assert!(!is_stem_path(Path::new("/x/vocals.opus")));
    }

    #[test]
    fn stem_mp4_opus_round_trip() {
        let rate: u32 = 48_000;
        let frames = (rate / 10) as usize;
        let tone = |amp: f32| -> Vec<f32> {
            (0..frames * 2)
                .map(|i| amp * ((i / 2) as f32 * 0.01).sin())
                .collect()
        };
        let mix = tone(0.1);
        let stems = [tone(0.2), tone(0.3), tone(0.4), tone(0.5)];
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.stem.mp4");
        encode_stem_mp4(
            &path,
            StemMuxFormat::Opus,
            rate,
            &mix,
            [&stems[0], &stems[1], &stems[2], &stems[3]],
            &StemAtom::default_ni(),
        )
        .unwrap();
        let bundle = decode_stem_file(&path).unwrap();
        assert_eq!(bundle.stems.len(), 4);
        assert!(bundle.mixdown.len() > 1000);
        // The declared pre-skip is honoured, so playback starts at input frame 0
        // rather than on the encoder's warm-up.
        assert_eq!(
            bundle.mixdown.len(),
            (frames - super::OPUS_PRE_SKIP as usize) * 2
        );
        // The temp file is renamed, never left behind.
        assert!(!path.with_extension("tmp").exists());
        assert_eq!(bundle.sample_rate, rate);
        assert_consistent(&bundle);
        // Amplitude per stream identifies its slot: NI order survived the mux.
        assert_amplitude_order(&bundle, 0.05);
        assert_eq!(bundle.atom.unwrap().stems[0].name, "Drums");
    }

    #[test]
    fn stem_mp4_flac_round_trip() {
        let rate = 44_100;
        let frames = rate / 10;
        let tone = |amp: f32, n: usize| -> Vec<f32> {
            (0..n * 2)
                .map(|i| amp * ((i / 2) as f32 * 0.01).sin())
                .collect()
        };
        let mix = tone(0.1, frames);
        // A longer stem must be truncated to the shortest stream, not padded.
        let stems = [
            tone(0.2, frames),
            tone(0.3, frames + 777),
            tone(0.4, frames),
            tone(0.5, frames),
        ];
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.stem.mp4");
        encode_stem_mp4(
            &path,
            StemMuxFormat::Flac,
            rate as u32,
            &mix,
            [&stems[0], &stems[1], &stems[2], &stems[3]],
            &StemAtom::default_ni(),
        )
        .unwrap();

        let bundle = decode_stem_file(&path).unwrap();
        assert_eq!(bundle.sample_rate, rate as u32);
        assert_eq!(bundle.mixdown.len(), frames * 2);
        assert_consistent(&bundle);
        // FLAC is lossless bar the 16-bit quantisation, so amplitudes are exact.
        assert_amplitude_order(&bundle, 0.005);
        assert_eq!(bundle.atom.unwrap().stems[3].name, "Vocals");
    }

    #[test]
    fn stem_mp4_rejects_bad_input() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.stem.mp4");
        let pcm = vec![0.1f32; 960 * 2];
        let stems = [&pcm[..], &pcm[..], &pcm[..], &pcm[..]];

        // Opus decodes at 48 kHz only; callers resample before muxing.
        let wrong_rate =
            encode_stem_mp4(&path, StemMuxFormat::Opus, 44_100, &pcm, stems, &atom()).unwrap_err();
        assert!(wrong_rate.to_string().contains("48000"), "{wrong_rate}");

        let mono = vec![0.1f32; 961];
        let odd =
            encode_stem_mp4(&path, StemMuxFormat::Flac, 44_100, &mono, stems, &atom()).unwrap_err();
        assert!(odd.to_string().contains("interleaved stereo"), "{odd}");

        let empty =
            encode_stem_mp4(&path, StemMuxFormat::Flac, 44_100, &[], stems, &atom()).unwrap_err();
        assert!(empty.to_string().contains("empty"), "{empty}");

        assert!(!path.exists(), "no file is written when input is rejected");
    }

    fn atom() -> StemAtom {
        StemAtom::default_ni()
    }

    /// Consistent streams: every track decoded the same number of stereo frames.
    fn assert_consistent(bundle: &super::StemPcmBundle) {
        for (index, stem) in bundle.stems.iter().enumerate() {
            assert_eq!(
                stem.len(),
                bundle.mixdown.len(),
                "stem {index} length differs from the mixdown"
            );
        }
    }

    /// The five streams were encoded at 0.1/0.2/0.3/0.4/0.5 peak amplitude; check
    /// each decoded stream still carries its own, so nothing was swapped.
    fn assert_amplitude_order(bundle: &super::StemPcmBundle, tolerance: f32) {
        let peak = |pcm: &[f32]| pcm.iter().fold(0.0f32, |max, s| max.max(s.abs()));
        for (index, expected) in [0.1, 0.2, 0.3, 0.4, 0.5].into_iter().enumerate() {
            let pcm = if index == 0 {
                &bundle.mixdown
            } else {
                &bundle.stems[index - 1]
            };
            let got = peak(pcm);
            assert!(
                (got - expected).abs() < tolerance,
                "stream {index} peak {got} is not {expected}"
            );
        }
    }
}
