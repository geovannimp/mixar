//! [`LoadableAudio`] implementations for library sources.

use audio_core::{LoadableAudio, LoadedAudio};
use codec::AudioDecoder;

use crate::source::{AudioSource, FileAudioSource, StreamAudioSource};

impl LoadableAudio for FileAudioSource {
    fn load(&self) -> anyhow::Result<LoadedAudio> {
        if !self.path().exists() {
            return Err(anyhow::anyhow!(
                "Audio file not found: {}",
                self.path().display()
            ));
        }

        let mut decoder = AudioDecoder::from_file(self.path())?;
        let sample_rate = decoder.sample_rate();
        let channels = decoder.channels();
        let samples = decoder.load_entire_file()?;
        let source_id = self.id.as_str().to_string();

        Ok(LoadedAudio {
            samples,
            sample_rate,
            channels,
            source_id,
        })
    }
}

impl LoadableAudio for StreamAudioSource {
    fn load(&self) -> anyhow::Result<LoadedAudio> {
        Err(anyhow::anyhow!(
            "streaming playback not implemented: {}",
            self.uri()
        ))
    }
}

impl LoadableAudio for AudioSource {
    fn load(&self) -> anyhow::Result<LoadedAudio> {
        match self {
            Self::File(s) => s.load(),
            Self::Stream(s) => s.load(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{TrackId, TrackMetadata};
    use std::path::PathBuf;

    #[test]
    fn file_audio_source_missing_file() {
        let source = FileAudioSource::new(
            TrackId::new("missing"),
            PathBuf::from("/no/such/file.wav"),
            TrackMetadata::default(),
        );
        let err = source.load().unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn stream_audio_source_not_implemented() {
        let source = StreamAudioSource::new(
            TrackId::new("stream:test"),
            "https://example.com/track.mp3",
            TrackMetadata::default(),
            None,
        );
        let err = source.load().unwrap_err();
        assert!(err
            .to_string()
            .contains("streaming playback not implemented"));
    }

    #[test]
    fn library_source_delegates_to_file() {
        let source = AudioSource::File(FileAudioSource::new(
            TrackId::new("missing"),
            PathBuf::from("/no/such/file.wav"),
            TrackMetadata::default(),
        ));
        let err = source.load().unwrap_err();
        assert!(err.to_string().contains("not found"));
    }
}
