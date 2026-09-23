use crate::{codec_registry, AudioDecoder, StemAtom};
use anyhow::{bail, Result};
use audio_core::Sample;
use std::fs::File;
use std::path::Path;
use symphonia::core::codecs::{Decoder, DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::{MetadataOptions, MetadataRevision, Value};
use symphonia::core::probe::Hint;

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
    let atom = format
        .metadata()
        .skip_to_latest()
        .and_then(stem_atom_from_metadata);

    let decoder_options = DecoderOptions::default();
    let mut tracks: Vec<(u32, u32, Box<dyn Decoder>)> = format
        .tracks()
        .iter()
        .filter(|track| track.codec_params.codec != CODEC_TYPE_NULL)
        .filter_map(|track| {
            codec_registry()
                .make(&track.codec_params, &decoder_options)
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
    use super::is_stem_path;
    use std::path::Path;

    #[test]
    fn is_stem_path_detects_extension() {
        assert!(is_stem_path(Path::new("/x/Track.stem.mp4")));
        assert!(is_stem_path(Path::new("/x/Track.STEM.MP4")));
        assert!(!is_stem_path(Path::new("/x/Track.mp4")));
        assert!(!is_stem_path(Path::new("/x/vocals.opus")));
    }
}
