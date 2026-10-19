//! Export source: the analysis, waveform tiers and cue points format adapters
//! need when writing out to a device or another application's layout.
//!
//! Adapters depend on [`library_core::ExportSource`], not on this manager, so
//! the DSP-adjacent work (decoding, waveform tiers, beat grids, cue reads) stays
//! here and format code stays in `library-adapters`. Embedded cover art is
//! re-encoded here too, at [`EXPORT_ARTWORK_EDGE_PX`].
//!
//! [`ExportSource`] is implemented twice. Directly on the manager, for callers
//! that already own it (tests, in-process one-shots). And on [`ExportSession`],
//! which is what a host hands an exporter when the manager lives behind a
//! `Mutex`: the session keeps the lock only for database reads and decode-cache
//! access, so file probing, decoding, DSP and cover encoding never block the
//! rest of the library.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use audio_core::{
    compute_full_track_envelope, compute_overview_envelope, LoadableAudio, LoadedAudio,
    SpectralPeak, WaveformAnalysisConfig,
};
use sea_orm::EntityTrait;

use library_core::{
    ExportArtwork, ExportBeatGrid, ExportCue, ExportSource, FileAudioSource, Library, LibraryError,
    Result, TrackExportData, TrackId, EXPORT_ARTWORK_EDGE_PX, EXPORT_WAVEFORM_ENTRIES_PER_SECOND,
};

use crate::db;
use crate::entity::TrackEntity;
use crate::waveform::BeatGridSnapshot;
use crate::LibraryManager;

impl ExportSource for LibraryManager {
    /// Collect everything an exporter needs for one track.
    ///
    /// Returns `Ok(None)` for stream tracks and for tracks whose file is
    /// missing; exporters report those themselves and fall back to file tags.
    /// The detail waveform tier is computed here (the library stores a
    /// resolution meant for its own displays), which decodes the file once —
    /// reused from the manager's decode cache when it is warm, though a one-shot
    /// call does not fill that cache: it must not evict PCM a deck is using.
    /// Long runs go through [`ExportSession`], which shares the cache both ways.
    fn export_data(&self, id: &TrackId) -> Result<Option<TrackExportData>> {
        let Some(facts) = self.export_facts(id)? else {
            return Ok(None);
        };

        let probe = probe_export_file(&facts.path);
        let audio = match facts.cached_audio.clone() {
            Some(cached) => cached,
            None => decode_export_audio(&facts.path)?,
        };
        let artwork = encode_export_artwork(probe.picture.as_deref());
        Ok(Some(assemble_export_data(
            &facts,
            &audio,
            probe.bit_depth.map(u16::from),
            artwork,
        )))
    }
}

/// Library-side facts about one track, gathered while the manager lock is held.
///
/// Everything here is a database read or a decode-cache lookup, so it stays
/// cheap: the file probing, decoding and DSP that turn these facts into
/// [`TrackExportData`] run after the caller drops the lock.
pub(crate) struct TrackExportFacts {
    /// Path of the track's audio file.
    pub path: PathBuf,
    /// BPM from the file's own metadata, before any grid fallback.
    pub file_bpm: Option<f64>,
    /// Date the track entered the library as `YYYY-MM-DD`.
    pub added_at: Option<String>,
    /// Stored beat grid, when the track has been analyzed.
    pub beat_grid: Option<BeatGridSnapshot>,
    /// Stored overview waveform peaks, when the track has one.
    pub waveform_overview: Option<Vec<SpectralPeak>>,
    /// Hot cues, saved loops and memory cues in export shape.
    pub cues: Vec<ExportCue>,
    /// Decoded PCM, when the shared decode cache already held this track.
    pub cached_audio: Option<Arc<LoadedAudio>>,
}

impl LibraryManager {
    /// Gather the library-side facts an export needs for one track.
    ///
    /// `Ok(None)` when the track is a stream or its file is gone — the same
    /// cases [`ExportSource::export_data`] reports as "nothing to export".
    /// Takes no lock of its own: callers that own a `Mutex<Self>` hold it for
    /// this call only.
    pub(crate) fn export_facts(&self, id: &TrackId) -> Result<Option<TrackExportFacts>> {
        let Some(source) = self.get_track(id)? else {
            return Ok(None);
        };
        let Some(file) = source.file() else {
            return Ok(None);
        };
        if !file.path().is_file() {
            return Ok(None);
        }

        // One grid read, used both for the exported grid and as the BPM of last
        // resort when neither tags nor a previous step supplied one.
        let beat_grid = self.get_track_beat_grid(id)?;
        let waveform_overview = self.get_track_waveform_overview(id)?.map(|row| row.peaks);
        let cached_audio = {
            let mut cache = Self::lock_decode_cache(&self.decode_cache)?;
            cache.get(id)
        };

        Ok(Some(TrackExportFacts {
            path: file.path().to_path_buf(),
            file_bpm: file.metadata.bpm,
            added_at: self.track_added_at(id)?,
            cues: self.cues_for_export(id, grid_bpm(beat_grid.as_ref()))?,
            beat_grid,
            waveform_overview,
            cached_audio,
        }))
    }

    /// Date the track entered the library, as `YYYY-MM-DD`.
    fn track_added_at(&self, id: &TrackId) -> Result<Option<String>> {
        let row = TrackEntity::find_by_id(id.as_str())
            .one(self.db.conn()?.as_connection())
            .map_err(db::db_err)?;
        let Some(row) = row else {
            return Ok(None);
        };
        Ok(row.added_at.parse::<i64>().ok().and_then(|secs| {
            chrono::DateTime::from_timestamp(secs, 0)
                .map(|instant| instant.date_naive().format("%Y-%m-%d").to_string())
        }))
    }

    /// Hot cues, then saved loops, in the shape device formats store them.
    ///
    /// Beat-quantized hot loops get an explicit loop end when a BPM is known,
    /// so targets that cannot express beat lengths still loop correctly. `bpm` is
    /// the caller's already-read grid tempo, so this does not re-read the grid.
    fn cues_for_export(&self, id: &TrackId, bpm: Option<f64>) -> Result<Vec<ExportCue>> {
        let mut cues = Vec::new();
        for cue in self.list_track_hot_cues(id)? {
            let position_ms = cue.position_ms.max(0) as u32;
            let loop_length_beats = cue.loop_length_beats.filter(|beats| *beats > 0);
            let loop_end_ms = match (loop_length_beats, bpm) {
                (Some(beats), Some(bpm)) => {
                    let beat_ms = 60_000.0 / bpm;
                    Some(position_ms + (beat_ms * f64::from(beats)).round() as u32)
                }
                _ => None,
            };
            cues.push(ExportCue {
                hot_cue_slot: Some(cue.slot_index),
                position_ms,
                loop_end_ms,
                loop_length_beats: loop_length_beats.map(|beats| beats as u32),
                label: non_empty(cue.label),
                color: non_empty(cue.color),
            });
        }
        for saved in self.list_track_loops(id)? {
            cues.push(ExportCue {
                hot_cue_slot: None,
                position_ms: saved.in_ms.max(0) as u32,
                loop_end_ms: Some(saved.out_ms.max(0) as u32),
                loop_length_beats: None,
                label: non_empty(saved.label),
                color: non_empty(saved.color),
            });
        }
        Ok(cues)
    }
}

/// Export source over a manager that lives behind its host lock.
///
/// Format adapters are handed `&dyn ExportSource`; the host owns the manager
/// inside an `Arc<Mutex<…>>`. The session locks the manager **briefly**, and
/// only for the parts that need it — the database reads and the decode cache —
/// while file probing, decode, envelope computation and cover encoding run with
/// the lock released, so a long export does not freeze browsing.
///
/// A session is one export run: decoded PCM is shared with the manager's decode
/// cache (so a track the player already decoded is never decoded again), and
/// encoded cover thumbnails are cached for the run, because an album repeats the
/// same multi-megapixel cover on every track.
pub struct ExportSession<'a> {
    manager: &'a Mutex<LibraryManager>,
    /// [`EXPORT_ARTWORK_EDGE_PX`] JPEGs keyed by a hash of the raw picture bytes,
    /// `None` for a cover we could not decode (so a broken cover is attempted
    /// once per run too).
    artwork_cache: Mutex<HashMap<u64, Option<Vec<u8>>>>,
    /// Covers actually encoded, i.e. cache misses; test-only instrumentation for
    /// asserting that a shared cover costs one encode per run.
    #[cfg(test)]
    artwork_encodes: std::sync::atomic::AtomicUsize,
}

impl<'a> ExportSession<'a> {
    /// Borrow a host-held manager as an export source.
    pub fn new(manager: &'a Mutex<LibraryManager>) -> Self {
        Self {
            manager,
            artwork_cache: Mutex::new(HashMap::new()),
            #[cfg(test)]
            artwork_encodes: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, LibraryManager>> {
        self.manager.lock().map_err(|_| LibraryError::Backend {
            backend: "export",
            message: "library lock poisoned".to_string(),
        })
    }

    /// Encoded cover thumbnail for one track, reusing this run's cache.
    ///
    /// Keyed by a hash of the raw embedded picture: every track of an album
    /// carries the same bytes, and turning one full-resolution cover into a
    /// thumbnail is the most expensive thing an export does per track.
    fn artwork_thumbnail(&self, picture: Option<&[u8]>) -> Option<ExportArtwork> {
        let picture = picture?;

        let key = picture_key(picture);
        let cached = self
            .artwork_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(&key)
            .cloned();
        if let Some(encoded) = cached {
            return encoded.map(|jpeg| ExportArtwork { jpeg });
        }

        // Encoding runs without the cache lock: it is the slow part, and a
        // racing duplicate encode only costs one wasted thumbnail.
        let encoded = codec::encode_artwork_thumbnail(picture, EXPORT_ARTWORK_EDGE_PX);
        #[cfg(test)]
        self.artwork_encodes
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.artwork_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(key, encoded.clone());
        encoded.map(|jpeg| ExportArtwork { jpeg })
    }
}

impl Library for ExportSession<'_> {
    fn name(&self) -> &'static str {
        "library"
    }

    fn get_track(&self, id: &TrackId) -> Result<Option<library_core::AudioSource>> {
        self.lock()?.get_track(id)
    }

    fn list_collections(&self) -> Result<Vec<library_core::Collection>> {
        self.lock()?.list_collections()
    }

    fn get_collection(
        &self,
        id: &library_core::CollectionId,
    ) -> Result<Option<library_core::Collection>> {
        self.lock()?.get_collection(id)
    }

    fn list_collection_tracks(
        &self,
        collection_id: &library_core::CollectionId,
    ) -> Result<Vec<library_core::AudioSource>> {
        self.lock()?.list_collection_tracks(collection_id)
    }

    fn list_collection_entries(
        &self,
        collection_id: &library_core::CollectionId,
    ) -> Result<Vec<library_core::CollectionEntry>> {
        self.lock()?.list_collection_entries(collection_id)
    }
}

impl ExportSource for ExportSession<'_> {
    /// Export data for one track, with the manager lock held only around the
    /// database reads and the decode-cache hit that start and end it.
    fn export_data(&self, id: &TrackId) -> Result<Option<TrackExportData>> {
        let facts = { self.lock()?.export_facts(id)? };
        let Some(facts) = facts else {
            return Ok(None);
        };

        // Everything below runs with the lock released.
        let probe = probe_export_file(&facts.path);
        let audio = match facts.cached_audio.clone() {
            Some(cached) => cached,
            None => {
                let loaded = decode_export_audio(&facts.path)?;
                // Re-check under a brief lock, mirroring
                // `prepare_source_for_playback`: a racing decode wins and the
                // duplicate `Arc` is dropped rather than replacing warm PCM.
                let lib = self.lock()?;
                let mut cache = LibraryManager::lock_decode_cache(&lib.decode_cache)?;
                if let Some(existing) = cache.get(id) {
                    existing
                } else {
                    cache.insert(id.clone(), loaded)
                }
            }
        };
        let artwork = self.artwork_thumbnail(probe.picture.as_deref());

        Ok(Some(assemble_export_data(
            &facts,
            &audio,
            probe.bit_depth.map(u16::from),
            artwork,
        )))
    }
}

/// Decode a track's file to PCM for an export.
fn decode_export_audio(path: &Path) -> Result<Arc<LoadedAudio>> {
    FileAudioSource::from_path(path)
        .load()
        .map(Arc::new)
        .map_err(|err| LibraryError::Backend {
            backend: "export",
            message: format!("{}: {err}", crate::path_label(path)),
        })
}

/// Probe one file for the two things export reads out of its tags.
///
/// One probe covers both, because parsing the container twice per track is pure
/// waste. Best-effort: an unreadable file yields "no cover, no bit depth" rather
/// than failing the track's export.
fn probe_export_file(path: &Path) -> codec::ArtworkProbe {
    match codec::read_file_artwork_probe(path) {
        Ok(probe) => probe,
        Err(err) => {
            tracing::debug!(
                path = %path.display(),
                error = %err,
                "export tag probe failed; exporting without embedded extras"
            );
            codec::ArtworkProbe::default()
        }
    }
}

/// Cover thumbnail an exporter may write, at [`EXPORT_ARTWORK_EDGE_PX`].
///
/// Best-effort: a file without artwork, with a cover format we cannot decode, or
/// with a cover bigger than the decoder's limits yields `None` — losing a cover
/// must not fail the track's export.
fn encode_export_artwork(picture: Option<&[u8]>) -> Option<ExportArtwork> {
    picture
        .and_then(|picture| codec::encode_artwork_thumbnail(picture, EXPORT_ARTWORK_EDGE_PX))
        .map(|jpeg| ExportArtwork { jpeg })
}

/// Build one track's export data from gathered facts, PCM and cover.
///
/// Shared by the manager path and [`ExportSession`] so both produce the same
/// fields with the same precedence: file BPM, then the stored grid, then the
/// grid BPM as the last resort.
fn assemble_export_data(
    facts: &TrackExportFacts,
    audio: &LoadedAudio,
    bit_depth: Option<u16>,
    artwork: Option<ExportArtwork>,
) -> TrackExportData {
    let config = WaveformAnalysisConfig::default();
    let mut data = TrackExportData {
        bpm: facts.file_bpm,
        cues: facts.cues.clone(),
        artwork,
        bit_depth,
        added_at: facts.added_at.clone(),
        ..TrackExportData::default()
    };

    if let Some(grid) = &facts.beat_grid {
        if data.bpm.is_none() {
            data.bpm = grid.bpm;
        }
        data.beat_grid = Some(ExportBeatGrid {
            beats: grid.beats.clone(),
            beats_per_bar: grid.beats_per_bar.max(1),
        });
    }
    if data.bpm.is_none() {
        data.bpm = grid_bpm(facts.beat_grid.as_ref());
    }

    // The library's own overview is display resolution; only synthesize one when
    // the track has no stored row (never analyzed, or cache cleared).
    data.waveform_overview = match &facts.waveform_overview {
        Some(peaks) if !peaks.is_empty() => peaks.clone(),
        _ => compute_overview_envelope(audio, &config),
    };
    data.waveform_detail = compute_full_track_envelope(audio, detail_buckets(audio), &config);
    data
}

/// BPM from a stored beat grid, when it is usable as a tempo.
fn grid_bpm(grid: Option<&BeatGridSnapshot>) -> Option<f64> {
    grid.and_then(|grid| grid.bpm)
        .filter(|bpm| bpm.is_finite() && *bpm > 0.0)
}

/// Per-run artwork cache key: 64-bit hash of the raw picture bytes.
///
/// The cache lives for one export and is never persisted, so a collision would
/// only mean one track borrowing another's cover inside a single run — and
/// brute-forcing one through SipHash is far more work than crafting the cover
/// itself, which the decoder limits above would refuse anyway.
fn picture_key(picture: &[u8]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    picture.hash(&mut hasher);
    hasher.finish()
}

/// Keep only non-blank strings.
fn non_empty(value: Option<String>) -> Option<String> {
    value.filter(|value| !value.trim().is_empty())
}

/// Detail-tier bucket count for a decoded track: 150 entries per second.
fn detail_buckets(audio: &audio_core::LoadedAudio) -> usize {
    let channels = usize::from(audio.channels.max(1));
    let frames = audio.samples.len() / channels;
    if frames == 0 {
        return 0;
    }
    let seconds = frames as f64 / f64::from(audio.sample_rate.max(1));
    (seconds * f64::from(EXPORT_WAVEFORM_ENTRIES_PER_SECOND)).round() as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io::Write;

    use crate::LibraryConfig;
    use lofty::config::WriteOptions;
    use lofty::picture::{MimeType, Picture, PictureType};
    use lofty::prelude::*;
    use lofty::probe::Probe;
    use lofty::tag::{Tag, TagType};

    #[test]
    fn detail_buckets_follow_the_export_rate() {
        let audio = audio_core::LoadedAudio {
            samples: vec![0.0; 48_000 * 2 * 3],
            sample_rate: 48_000,
            channels: 2,
            source_id: "test".into(),
        };
        assert_eq!(detail_buckets(&audio), 450);
    }

    #[test]
    fn empty_audio_asks_for_no_buckets() {
        let audio = audio_core::LoadedAudio {
            samples: Vec::new(),
            sample_rate: 44_100,
            channels: 2,
            source_id: "test".into(),
        };
        assert_eq!(detail_buckets(&audio), 0);
    }

    #[test]
    fn blank_labels_are_dropped() {
        assert_eq!(non_empty(Some("  ".to_string())), None);
        assert_eq!(non_empty(None), None);
        assert_eq!(
            non_empty(Some("Intro".to_string())),
            Some("Intro".to_string())
        );
    }

    /// 1s of 16-bit mono PCM at 8 kHz: decodable, and short enough that the
    /// export's detail waveform is exactly 150 buckets.
    fn write_wav(path: &Path) {
        let sample_rate = 8_000u32;
        let pcm = vec![0u8; sample_rate as usize * 2];
        let data_size = pcm.len() as u32;
        let mut file = std::fs::File::create(path).unwrap();
        file.write_all(b"RIFF").unwrap();
        file.write_all(&(36 + data_size).to_le_bytes()).unwrap();
        file.write_all(b"WAVEfmt ").unwrap();
        file.write_all(&16u32.to_le_bytes()).unwrap();
        file.write_all(&1u16.to_le_bytes()).unwrap();
        file.write_all(&1u16.to_le_bytes()).unwrap();
        file.write_all(&sample_rate.to_le_bytes()).unwrap();
        file.write_all(&(sample_rate * 2).to_le_bytes()).unwrap();
        file.write_all(&2u16.to_le_bytes()).unwrap();
        file.write_all(&16u16.to_le_bytes()).unwrap();
        file.write_all(b"data").unwrap();
        file.write_all(&data_size.to_le_bytes()).unwrap();
        file.write_all(&pcm).unwrap();
    }

    /// PNG of `width`×`height`; a real decoder must scale it for a thumbnail.
    fn cover_png(width: u32, height: u32, red: u8) -> Vec<u8> {
        let rgb = image::RgbImage::from_fn(width, height, |x, y| {
            image::Rgb([red, (x % 256) as u8, (y % 256) as u8])
        });
        let mut bytes = Vec::new();
        image::DynamicImage::ImageRgb8(rgb)
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Png,
            )
            .unwrap();
        bytes
    }

    /// Write a WAV whose front cover is `cover`, asserting lofty reads it back.
    fn write_wav_with_cover(path: &Path, cover: Vec<u8>) {
        write_wav(path);

        let mut tagged = Probe::open(path).unwrap().read().unwrap();
        if tagged.primary_tag().is_none() {
            tagged.insert_tag(Tag::new(TagType::Id3v2));
        }
        tagged.primary_tag_mut().expect("id3v2 tag").set_picture(
            0,
            Picture::new_unchecked(PictureType::CoverFront, Some(MimeType::Png), None, cover),
        );
        tagged
            .save_to_path(path, WriteOptions::default())
            .expect("save artwork tag");

        let reread = Probe::open(path).unwrap().read().unwrap();
        assert_eq!(
            reread.primary_tag().expect("id3v2 tag").pictures().len(),
            1,
            "lofty must read back the picture it wrote"
        );
    }

    /// The session's library helpers are shared by the tests below.
    struct Fixture {
        library: Mutex<LibraryManager>,
        _dir: tempfile::TempDir,
    }

    fn fixture(tracks: &[(&str, Option<Vec<u8>>)]) -> (Fixture, Vec<TrackId>) {
        let dir = tempfile::tempdir().unwrap();
        let library = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
        let mut ids = Vec::new();
        for (name, cover) in tracks {
            let path = dir.path().join(name);
            match cover {
                Some(cover) => write_wav_with_cover(&path, cover.clone()),
                None => write_wav(&path),
            }
            ids.push(library.import_file_path(&path).unwrap().id().clone());
        }
        (
            Fixture {
                library: Mutex::new(library),
                _dir: dir,
            },
            ids,
        )
    }

    #[test]
    fn second_export_of_a_track_reuses_the_cached_decode() {
        let (fixture, ids) = fixture(&[("track.wav", None)]);
        let id = &ids[0];
        let session = ExportSession::new(&fixture.library);

        let first = session.export_data(id).unwrap().expect("export data");
        assert_eq!(
            first.waveform_detail.len(),
            150,
            "1s at 150 entries/second, computed from the file"
        );
        assert!(fixture
            .library
            .lock()
            .unwrap()
            .decode_cache
            .lock()
            .unwrap()
            .contains(id));

        // Corrupt the file: a second call that decoded again would fail here,
        // while a cache hit returns the PCM the first call already decoded.
        std::fs::write(fixture._dir.path().join("track.wav"), b"not audio").unwrap();

        let second = session.export_data(id).unwrap().expect("export data");
        assert_eq!(second.waveform_detail, first.waveform_detail);
        assert_eq!(second.waveform_overview, first.waveform_overview);
        assert_eq!(second.bpm, first.bpm);
    }

    #[test]
    fn one_cover_is_encoded_once_per_session() {
        let shared = cover_png(400, 300, 10);
        let other = cover_png(400, 300, 200);
        let (fixture, ids) = fixture(&[
            ("one.wav", Some(shared.clone())),
            ("two.wav", Some(shared)),
            ("three.wav", Some(other)),
        ]);
        let session = ExportSession::new(&fixture.library);

        let first = session.export_data(&ids[0]).unwrap().unwrap();
        let second = session.export_data(&ids[1]).unwrap().unwrap();
        let first_art = first.artwork.clone().expect("cover art");
        assert!(first_art.jpeg.starts_with(&[0xFF, 0xD8]), "JPEG (SOI)");
        assert_eq!(first.artwork, second.artwork, "album shares one cover");
        assert_eq!(
            session
                .artwork_encodes
                .load(std::sync::atomic::Ordering::Relaxed),
            1,
            "the shared cover must be decoded once, not once per track"
        );
        assert_eq!(session.artwork_cache.lock().unwrap().len(), 1);

        // A different cover is a miss, and is cached like the first.
        let third = session.export_data(&ids[2]).unwrap().unwrap();
        assert_ne!(third.artwork, first.artwork);
        assert_eq!(
            session
                .artwork_encodes
                .load(std::sync::atomic::Ordering::Relaxed),
            2
        );
        assert_eq!(session.artwork_cache.lock().unwrap().len(), 2);
    }

    #[test]
    fn cached_artwork_is_the_export_sized_thumbnail() {
        let (fixture, ids) = fixture(&[("track.wav", Some(cover_png(400, 300, 30)))]);
        let session = ExportSession::new(&fixture.library);

        // Twice: the resized bytes come from the cache the second time.
        for _ in 0..2 {
            let data = session.export_data(&ids[0]).unwrap().unwrap();
            let jpeg = data.artwork.expect("cover art").jpeg;
            let decoded = image::load_from_memory(&jpeg).unwrap();
            assert_eq!(
                decoded.width().max(decoded.height()),
                EXPORT_ARTWORK_EDGE_PX,
                "cached artwork keeps the export edge"
            );
        }
    }

    #[test]
    fn track_without_cover_has_no_artwork() {
        let (fixture, ids) = fixture(&[("bare.wav", None)]);
        let session = ExportSession::new(&fixture.library);

        let data = session.export_data(&ids[0]).unwrap().expect("export data");
        assert!(data.artwork.is_none());
        assert!(session.artwork_cache.lock().unwrap().is_empty());
    }

    #[test]
    fn session_delegates_the_library_reads() {
        let (fixture, ids) = fixture(&[("track.wav", None)]);
        let session = ExportSession::new(&fixture.library);

        assert_eq!(session.name(), "library");
        assert!(session.get_track(&ids[0]).unwrap().is_some());
        assert!(session.list_collections().unwrap().is_empty());
        assert!(session
            .get_track(&TrackId::new("/no/such/track"))
            .unwrap()
            .is_none());
    }

    #[test]
    fn missing_file_and_stream_tracks_export_nothing() {
        let (fixture, ids) = fixture(&[("track.wav", None)]);
        let session = ExportSession::new(&fixture.library);
        std::fs::remove_file(fixture._dir.path().join("track.wav")).unwrap();

        assert!(session.export_data(&ids[0]).unwrap().is_none());
    }
}
