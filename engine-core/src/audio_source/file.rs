use super::source::{AudioSource, LoadedAudio};
use anyhow::Result;
use codec::AudioDecoder;
use std::path::{Path, PathBuf};

/// Audio source that loads from a file on disk.
#[derive(Debug, Clone)]
pub struct FileAudioSource {
    path: PathBuf,
}

impl FileAudioSource {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl AudioSource for FileAudioSource {
    fn load(&self) -> Result<LoadedAudio> {
        if !self.path.exists() {
            return Err(anyhow::anyhow!(
                "Audio file not found: {}",
                self.path.display()
            ));
        }

        let mut decoder = AudioDecoder::from_file(&self.path)?;
        let sample_rate = decoder.sample_rate();
        let channels = decoder.channels();
        let samples = decoder.load_entire_file()?;
        let source_id = self.path.to_string_lossy().into_owned();

        Ok(LoadedAudio {
            samples,
            sample_rate,
            channels,
            source_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_audio_source_not_found() {
        let source = FileAudioSource::new("nonexistent.wav");
        let err = source.load().unwrap_err();
        assert!(err.to_string().contains("not found"));
    }
}
